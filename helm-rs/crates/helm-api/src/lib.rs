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
use helm_proc::{
    HostMonitor, LogBuffer, MetricsCollector, ProcessManager, Scheduler, StatusBroadcaster,
};
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
        .merge(routers::stacks::router())
        .merge(routers::scripts::router())
        .merge(routers::faq::router())
        .merge(routers::diagnostics::router())
        .merge(routers::update::router())
        .merge(routers::settings::router())
        .merge(routers::themes::router())
        .merge(routers::ws::router())
        .fallback(fallback_404)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::pin::pin_auth,
        ))
        .with_state(state)
}

/// Start every service flagged `auto_start=1` at boot, honoring `depends_on`
/// ordering. Best-effort: a failed start is logged, never fatal. Services whose
/// in-scope dependencies form a cycle (or stay unmet) are skipped with a warning.
pub async fn autostart_services(state: &AppState) {
    let rows = match helm_db::repo::services::list(&state.db.pool).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("autostart: list services failed: {e}");
            return;
        }
    };
    let mut pending: Vec<_> = rows.into_iter().filter(|r| r.auto_start != 0).collect();
    if pending.is_empty() {
        return;
    }
    let ids: std::collections::HashSet<i64> = pending.iter().map(|r| r.id).collect();
    let mut started: std::collections::HashSet<i64> = std::collections::HashSet::new();

    // Iteratively start services whose auto_start dependencies are already up.
    // Deps outside the auto_start set do not gate ordering here — `spawn` still
    // validates they are running and fails the start otherwise.
    let mut progress = true;
    while progress && !pending.is_empty() {
        progress = false;
        let mut still = Vec::with_capacity(pending.len());
        for row in std::mem::take(&mut pending) {
            let deps: Vec<i64> = helm_db::repo::de_json::<Vec<i64>>(&row.depends_on)
                .ok()
                .flatten()
                .unwrap_or_default();
            let ready = deps
                .iter()
                .all(|d| !ids.contains(d) || started.contains(d));
            if !ready {
                still.push(row);
                continue;
            }
            let id = row.id;
            progress = true;
            match routers::services::build_spawn_spec(&row).await {
                Ok(spec) => match state.pm.spawn(spec).await {
                    Ok(mp) => {
                        tracing::info!("autostart: service {id} started (pid {})", mp.pid);
                        started.insert(id);
                    }
                    Err(e) => tracing::warn!("autostart: spawn service {id} failed: {e}"),
                },
                Err(e) => tracing::warn!("autostart: build spec for service {id} failed: {e}"),
            }
        }
        pending = still;
    }
    for row in pending {
        tracing::warn!(
            "autostart: service {} skipped — dependency cycle or unmet deps",
            row.id
        );
    }
}

/// Helper for helm-bin to construct fresh state without re-declaring fields.
#[allow(clippy::too_many_arguments)]
pub fn make_state(
    db: helm_db::Db,
    pm: Arc<ProcessManager>,
    log_buffer: Arc<LogBuffer>,
    status: Arc<StatusBroadcaster>,
    metrics: Arc<MetricsCollector>,
    host: Arc<HostMonitor>,
    scheduler: Arc<Scheduler>,
) -> AppState {
    AppState {
        db,
        started_at: Arc::new(Instant::now()),
        pm,
        log_buffer,
        status,
        metrics,
        host,
        scheduler,
        dashboard_pin: String::new(),
        restart_tx: None,
    }
}
