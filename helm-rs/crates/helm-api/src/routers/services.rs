use crate::{error::ApiError, state::AppState};
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use helm_core::models::{OutputLogEntry, ServiceCreate, ServiceResponse, ServiceUpdate};
use helm_db::repo::{output_logs, services};
use helm_platform::{build_rust_argv, build_rust_prebuild_argv, is_compatible, RunMode, RustSpec};
use helm_proc::SpawnSpec;
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/services", get(list).post(create))
        .route("/api/services/reorder", axum::routing::post(reorder))
        .route("/api/services/:id", get(get_one).put(update).delete(delete))
        .route("/api/services/:id/start", axum::routing::post(start))
        .route("/api/services/:id/stop", axum::routing::post(stop))
        .route("/api/services/:id/restart", axum::routing::post(restart))
        .route("/api/services/:id/logs", get(logs))
        .route("/api/services/:id/metrics", get(metrics))
}

fn resolve_status(state: &AppState, id: i64) -> (String, Option<i64>) {
    let key = format!("service_{id}");
    if let Some(mp) = state.pm.get(&key) {
        ("running".into(), Some(mp.pid as i64))
    } else {
        ("stopped".into(), None)
    }
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<ServiceResponse>>, ApiError> {
    let rows = services::list(&state.db.pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let (status, pid) = resolve_status(&state, row.id);
        out.push(row.into_response(status, pid)?);
    }
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
struct ReorderBody {
    ids: Vec<i64>,
}

async fn reorder(
    State(state): State<AppState>,
    Json(body): Json<ReorderBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    if body.ids.is_empty() {
        return Err(ApiError::BadRequest("ids must not be empty".into()));
    }
    services::reorder(&state.db.pool, &body.ids).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ServiceResponse>, ApiError> {
    let row = services::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Service {id} not found")))?;
    let (status, pid) = resolve_status(&state, id);
    Ok(Json(row.into_response(status, pid)?))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<ServiceCreate>,
) -> Result<(axum::http::StatusCode, Json<ServiceResponse>), ApiError> {
    let id = services::create(&state.db.pool, &body).await?;
    let row = services::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("create succeeded but get failed".into()))?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(row.into_response("stopped".into(), None)?),
    ))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ServiceUpdate>,
) -> Result<Json<ServiceResponse>, ApiError> {
    let exists = services::get(&state.db.pool, id).await?;
    if exists.is_none() {
        return Err(ApiError::NotFound(format!("Service {id} not found")));
    }
    services::update(&state.db.pool, id, &body).await?;
    let row = services::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("update succeeded but get failed".into()))?;
    let (status, pid) = resolve_status(&state, id);
    Ok(Json(row.into_response(status, pid)?))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<axum::http::StatusCode, ApiError> {
    let exists = services::get(&state.db.pool, id).await?;
    if exists.is_none() {
        return Err(ApiError::NotFound(format!("Service {id} not found")));
    }
    let key = format!("service_{id}");
    if state.pm.status(&key) == "running" {
        if let Err(e) = state.pm.terminate(&key).await {
            tracing::warn!("delete: stop service {id} failed: {e}");
        }
    }
    services::delete(&state.db.pool, id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn rust_spec_from(row: &helm_db::repo::services::ServiceRow) -> RustSpec {
    RustSpec {
        manifest_path: row.manifest_path.clone(),
        binary_path: row.binary_path.clone(),
        cargo_profile: row.cargo_profile.clone(),
        cargo_features: row.cargo_features.clone(),
        prebuild: row.prebuild != 0,
    }
}

async fn run_prebuild(argv: Vec<String>, cwd: Option<&str>) -> Result<(), ApiError> {
    let (head, tail) = argv
        .split_first()
        .ok_or_else(|| ApiError::Internal("empty prebuild argv".into()))?;
    let mut cmd = tokio::process::Command::new(head);
    cmd.args(tail);
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    let status = cmd
        .status()
        .await
        .map_err(|e| ApiError::BadRequest(format!("cargo build spawn failed: {e}")))?;
    if !status.success() {
        return Err(ApiError::BadRequest(format!(
            "cargo build failed (exit {:?})",
            status.code()
        )));
    }
    Ok(())
}

pub(crate) async fn build_spawn_spec(
    row: &helm_db::repo::services::ServiceRow,
) -> Result<SpawnSpec, ApiError> {
    use helm_db::repo::de_json;
    if !is_compatible(&row.platform) {
        return Err(ApiError::BadRequest(
            "Service not compatible with current platform".into(),
        ));
    }
    let args: Vec<String> = de_json::<Vec<String>>(&row.args)?.unwrap_or_default();
    let env: Option<std::collections::HashMap<String, String>> = de_json(&row.env)?;
    let depends_on: Vec<i64> = de_json::<Vec<i64>>(&row.depends_on)?.unwrap_or_default();

    let (command, final_args) = if row.r#type == "rust" {
        let rs = rust_spec_from(row);
        if let Some(prebuild) =
            build_rust_prebuild_argv(&rs).map_err(|e| ApiError::BadRequest(e.to_string()))?
        {
            run_prebuild(prebuild, row.cwd.as_deref()).await?;
        }
        let argv = build_rust_argv(&rs, &args).map_err(|e| ApiError::BadRequest(e.to_string()))?;
        let (head, tail) = argv
            .split_first()
            .ok_or_else(|| ApiError::Internal("empty rust argv".into()))?;
        (head.clone(), tail.to_vec())
    } else {
        let cmd = row
            .command
            .clone()
            .ok_or_else(|| ApiError::BadRequest("Service has no command configured".into()))?;
        (cmd, args)
    };

    Ok(SpawnSpec {
        entity_type: "service".into(),
        entity_id: row.id,
        command,
        args: final_args,
        cwd: row.cwd.clone(),
        env,
        venv_path: row.venv_path.clone(),
        run_mode: RunMode::Exec,
        restart_on_crash: row.restart_on_crash != 0,
        webhook_url: row.webhook_url.clone(),
        name: row.name.clone(),
        depends_on,
    })
}

async fn start(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = services::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Service {id} not found")))?;
    let spec = build_spawn_spec(&row).await?;
    match state.pm.spawn(spec).await {
        Ok(mp) => Ok(Json(
            serde_json::json!({"status": "started", "pid": mp.pid}),
        )),
        Err(e) => Err(map_spawn_error(e)),
    }
}

async fn stop(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let key = format!("service_{id}");
    match state.pm.terminate(&key).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "stopped"}))),
        Err(e) => Err(ApiError::Conflict(e.to_string())),
    }
}

async fn restart(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = services::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Service {id} not found")))?;
    let key = format!("service_{id}");
    if state.pm.status(&key) == "running" {
        if let Err(e) = state.pm.terminate(&key).await {
            tracing::warn!("restart: stop {id} failed: {e}");
        }
    }
    let spec = build_spawn_spec(&row).await?;
    match state.pm.spawn(spec).await {
        Ok(mp) => Ok(Json(
            serde_json::json!({"status": "restarted", "pid": mp.pid}),
        )),
        Err(e) => Err(map_spawn_error(e)),
    }
}

pub(crate) fn map_spawn_error(e: anyhow::Error) -> ApiError {
    let msg = e.to_string();
    if msg.contains("already running") || msg.contains("Dependency") {
        ApiError::Conflict(msg)
    } else {
        ApiError::BadRequest(format!("Failed to start: {msg}"))
    }
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}
fn default_limit() -> i64 {
    200
}

async fn logs(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<Vec<OutputLogEntry>>, ApiError> {
    let rows = output_logs::recent(&state.db.pool, "service", id, q.limit).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn metrics(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<helm_proc::MetricsSnapshot>>, ApiError> {
    let key = format!("service_{id}");
    Ok(Json(state.metrics.snapshots(&key)))
}
