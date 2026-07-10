//! Server-side key/value settings, persisted in the `settings` table.
//! Used for config that should be shared across browsers / addressable by the
//! MCP server (e.g. the dashboard theme), unlike the browser-local prefs.
//!
//! `GET /api/settings/{key}` — returns the stored JSON value, or `null`.
//! `PUT /api/settings/{key}` — stores the JSON request body verbatim.

use crate::{error::ApiError, state::AppState};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

static KEY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]{1,64}$").unwrap());

pub fn router() -> Router<AppState> {
    Router::new().route("/api/settings/:key", get(get_setting).put(put_setting))
}

fn check_key(key: &str) -> Result<(), ApiError> {
    if KEY_RE.is_match(key) {
        Ok(())
    } else {
        Err(ApiError::BadRequest("Invalid settings key".into()))
    }
}

async fn get_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<Value>, ApiError> {
    check_key(&key)?;
    let raw = state.db.get_setting(&key).await?;
    let value = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    Ok(Json(value))
}

async fn put_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(value): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    check_key(&key)?;
    let serialized =
        serde_json::to_string(&value).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    state.db.set_setting(&key, &serialized).await?;
    Ok(Json(value))
}
