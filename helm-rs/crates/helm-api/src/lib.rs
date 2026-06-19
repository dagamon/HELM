pub mod error;
pub mod middleware;
pub mod routers;
pub mod state;

use axum::{
    http::StatusCode,
    middleware as axum_middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use helm_proc::{LogBuffer, MetricsCollector, ProcessManager, Scheduler, StatusBroadcaster};
use serde_json::json;
use state::AppState;
use std::sync::Arc;
use std::time::Instant;

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}

async fn fallback_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({"detail": "Not Found"})))
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/login", post(routers::system::login))
        .route("/api/system/info", get(routers::system::info))
        .route("/api/export", get(routers::system::export))
        .route("/api/restart-server", post(routers::system::restart_server))
        .merge(routers::services::router())
        .merge(routers::scripts::router())
        .merge(routers::faq::router())
        .merge(routers::ws::router())
        .fallback(fallback_404)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::pin::pin_auth,
        ))
        .with_state(state)
}

/// Helper for helm-bin to construct fresh state without re-declaring fields.
#[allow(clippy::too_many_arguments)]
pub fn make_state(
    db: helm_db::Db,
    pm: Arc<ProcessManager>,
    log_buffer: Arc<LogBuffer>,
    status: Arc<StatusBroadcaster>,
    metrics: Arc<MetricsCollector>,
    scheduler: Arc<Scheduler>,
) -> AppState {
    AppState {
        db,
        started_at: Arc::new(Instant::now()),
        pm,
        log_buffer,
        status,
        metrics,
        scheduler,
        dashboard_pin: String::new(),
        restart_tx: None,
    }
}
