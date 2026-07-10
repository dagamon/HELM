//! Read-only theme catalog served from the repo-root `themes/` directory.
//!
//! Each `*.json` file is one theme:
//! `{ "name", "label", "hint", "colors": {token: hex}, "panels": {key: hex} }`.
//! Custom themes are added by dropping a JSON file into the folder — there is
//! no in-app theme editor. `panels` lists the card colors the dashboard may
//! use while this theme is active.
//!
//! `GET /api/themes` — every valid theme, sorted by file name.

use crate::{error::ApiError, state::AppState};
use axum::{routing::get, Json, Router};
use serde_json::Value;
use std::path::PathBuf;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/themes", get(list))
}

fn themes_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HELM_THEMES_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    // themes/ relative to workspace root (helm-rs/../themes)
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("themes")
}

fn is_valid_theme(v: &Value) -> bool {
    v.get("name").and_then(Value::as_str).is_some()
        && v.get("colors").map(Value::is_object).unwrap_or(false)
}

async fn list() -> Result<Json<Vec<Value>>, ApiError> {
    let dir = themes_dir();
    if !dir.exists() {
        return Ok(Json(Vec::new()));
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| ApiError::Internal(format!("read themes dir: {e}")))?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let raw = std::fs::read_to_string(entry.path()).unwrap_or_default();
        match serde_json::from_str::<Value>(&raw) {
            Ok(v) if is_valid_theme(&v) => out.push(v),
            _ => tracing::warn!("themes: skipping invalid file {:?}", entry.file_name()),
        }
    }
    Ok(Json(out))
}
