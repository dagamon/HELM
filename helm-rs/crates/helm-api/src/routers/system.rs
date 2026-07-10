use crate::{error::ApiError, state::AppState};
use axum::{
    extract::State,
    http::{header::SET_COOKIE, StatusCode},
    response::IntoResponse,
    Json,
};
use helm_core::models::ExportPayload;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Column, Row};

const PIN_COOKIE_NAME: &str = "helm_pin";

/// HELM release version, single-sourced from the workspace `Cargo.toml`.
pub const HELM_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub version: &'static str,
    pub runtime_version: String,
    pub platform: String,
    pub uptime_seconds: u64,
    pub service_count: i64,
    pub running_count: i64,
}

fn detect_platform() -> String {
    #[cfg(target_os = "windows")]
    {
        "windows".into()
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/version") {
            if text.to_lowercase().contains("microsoft") {
                return "wsl2".into();
            }
        }
        "linux".into()
    }
}

fn os_capitalised() -> String {
    let raw = std::env::consts::OS;
    let mut chars = raw.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

pub async fn info(State(state): State<AppState>) -> Result<Json<SystemInfo>, ApiError> {
    let service_count = state.db.service_count().await.unwrap_or(0);
    Ok(Json(SystemInfo {
        os: os_capitalised(),
        version: HELM_VERSION,
        runtime_version: format!("rustc {}", option_env!("RUSTC_VERSION").unwrap_or("stable")),
        platform: detect_platform(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        service_count,
        running_count: 0, // filled by ProcessManager in S6
    }))
}

const EXCLUDE_COLS: &[&str] = &["id", "created_at", "updated_at"];

pub async fn export(State(state): State<AppState>) -> Result<Json<ExportPayload>, ApiError> {
    let services = strip_rows_as_json(
        &state.db,
        "SELECT * FROM services ORDER BY id",
        &["args", "env", "tags", "depends_on"],
    )
    .await?;
    let scripts = strip_rows_as_json(
        &state.db,
        "SELECT * FROM scripts ORDER BY id",
        &["args", "tags"],
    )
    .await?;
    let stacks = strip_rows_as_json(&state.db, "SELECT * FROM stacks ORDER BY id", &["tags"]).await?;

    Ok(Json(ExportPayload {
        version: 1,
        services,
        scripts,
        stacks,
    }))
}

async fn strip_rows_as_json(
    db: &helm_db::Db,
    sql: &str,
    json_fields: &[&str],
) -> Result<Vec<Value>, ApiError> {
    let rows = sqlx::query(sql).fetch_all(&db.pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut map = serde_json::Map::new();
        for col in row.columns() {
            let name = col.name();
            if EXCLUDE_COLS.contains(&name) {
                continue;
            }
            let value = column_to_json(&row, name);
            // JSON fields: parse string back to structured value
            let final_val = if json_fields.contains(&name) {
                match &value {
                    Value::String(s) => serde_json::from_str(s).unwrap_or(Value::Null),
                    _ => value,
                }
            } else {
                value
            };
            map.insert(name.to_string(), final_val);
        }
        out.push(Value::Object(map));
    }
    Ok(out)
}

fn column_to_json(row: &sqlx::sqlite::SqliteRow, name: &str) -> Value {
    use sqlx::ValueRef;

    let raw = match row.try_get_raw(name) {
        Ok(v) => v,
        Err(_) => return Value::Null,
    };
    if raw.is_null() {
        return Value::Null;
    }
    if let Ok(v) = row.try_get::<i64, _>(name) {
        return Value::Number(v.into());
    }
    if let Ok(v) = row.try_get::<f64, _>(name) {
        return serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<String, _>(name) {
        return Value::String(v);
    }
    Value::Null
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub pin: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    if state.dashboard_pin.trim().is_empty() {
        return (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "auth_enabled": false})),
        )
            .into_response();
    }

    if payload.pin != state.dashboard_pin {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"detail": "Invalid PIN"})),
        )
            .into_response();
    }

    let cookie = format!(
        "{PIN_COOKIE_NAME}={}; Path=/; HttpOnly; SameSite=Lax",
        payload.pin
    );
    (
        StatusCode::OK,
        [(SET_COOKIE, cookie)],
        Json(serde_json::json!({"status": "ok"})),
    )
        .into_response()
}

pub async fn restart_server(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(tx) = &state.restart_tx {
        let _ = tx.send(true);
    }
    Json(serde_json::json!({"status": "restarting"}))
}
