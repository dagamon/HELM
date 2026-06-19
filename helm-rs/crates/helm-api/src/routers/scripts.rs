use crate::{error::ApiError, state::AppState};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use helm_core::models::{RunLog, ScriptCreate, ScriptResponse, ScriptUpdate};
use helm_db::repo::{de_json, run_logs, scripts};
use helm_platform::{is_compatible, RunMode};
use helm_proc::SpawnSpec;
use std::collections::HashMap;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/scripts", get(list).post(create))
        .route("/api/scripts/:id", get(get_one).put(update).delete(delete))
        .route("/api/scripts/:id/run", post(run))
        .route("/api/scripts/runs/:id", get(get_run))
        .route("/api/scripts/scheduler/next-run", get(next_runs))
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<ScriptResponse>>, ApiError> {
    let rows = scripts::list(&state.db.pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row.into_response()?);
    }
    Ok(Json(out))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<ScriptCreate>,
) -> Result<(axum::http::StatusCode, Json<ScriptResponse>), ApiError> {
    let id = scripts::create(&state.db.pool, &body).await?;
    let row = scripts::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("create succeeded but get failed".into()))?;
    Ok((axum::http::StatusCode::CREATED, Json(row.into_response()?)))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ScriptResponse>, ApiError> {
    let row = scripts::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Script {id} not found")))?;
    Ok(Json(row.into_response()?))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ScriptUpdate>,
) -> Result<Json<ScriptResponse>, ApiError> {
    if scripts::get(&state.db.pool, id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Script {id} not found")));
    }
    scripts::update(&state.db.pool, id, &body).await?;
    let row = scripts::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("update succeeded but get failed".into()))?;
    Ok(Json(row.into_response()?))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<axum::http::StatusCode, ApiError> {
    if scripts::get(&state.db.pool, id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Script {id} not found")));
    }
    scripts::delete(&state.db.pool, id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn run(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = scripts::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Script {id} not found")))?;
    if !is_compatible(&row.platform) {
        return Err(ApiError::BadRequest(
            "Script not compatible with current platform".into(),
        ));
    }
    let args: Option<Vec<String>> = de_json(&row.args)?;
    let run_mode = match row.run_mode.as_deref() {
        Some("shell") => RunMode::Shell,
        _ => RunMode::Exec,
    };
    let spec = SpawnSpec {
        entity_type: "script".into(),
        entity_id: row.id,
        command: row.command.clone(),
        args: args.unwrap_or_default(),
        cwd: row.cwd.clone(),
        env: None,
        venv_path: None,
        run_mode,
        restart_on_crash: false,
        webhook_url: None,
        name: row.name.clone(),
        depends_on: vec![],
    };
    match state.pm.spawn(spec).await {
        Ok(mp) => Ok(Json(serde_json::json!({
            "run_id": mp.run_log_id,
            "pid": mp.pid,
            "started_at": mp.started_at,
        }))),
        Err(e) => Err(ApiError::BadRequest(format!("Failed to run: {e}"))),
    }
}

async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<RunLog>, ApiError> {
    let row = run_logs::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Run {id} not found")))?;
    Ok(Json(row.into()))
}

async fn next_runs(
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, String>>, ApiError> {
    let mut out = HashMap::new();
    for (id, next) in state.scheduler.all_next_runs().await {
        if let Some(t) = next {
            out.insert(format!("script_{id}"), t.to_rfc3339());
        }
    }
    Ok(Json(out))
}
