use crate::state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{header::COOKIE, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

const PIN_COOKIE_NAME: &str = "helm_pin";

fn is_public_path(path: &str) -> bool {
    path == "/api/health"
        || path == "/api/login"
        || path == "/static"
        || path.starts_with("/static/")
}

fn pin_from_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let mut kv = part.trim().splitn(2, '=');
        let key = kv.next()?.trim();
        let value = kv.next().unwrap_or("").trim();
        if key == PIN_COOKIE_NAME {
            return Some(value.to_string());
        }
    }
    None
}

fn extract_pin(req: &Request<Body>) -> Option<String> {
    req.headers()
        .get("x-pin")
        .or_else(|| req.headers().get("x-dashboard-pin"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .or_else(|| pin_from_cookie(req.headers()))
}

pub async fn pin_auth(State(state): State<AppState>, req: Request<Body>, next: Next) -> Response {
    if state.dashboard_pin.trim().is_empty() {
        return next.run(req).await;
    }

    let path = req.uri().path();
    if is_public_path(path) || (!path.starts_with("/api") && !path.starts_with("/ws")) {
        return next.run(req).await;
    }

    if extract_pin(&req).as_deref() == Some(state.dashboard_pin.as_str()) {
        return next.run(req).await;
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"detail": "Invalid or missing PIN"})),
    )
        .into_response()
}
