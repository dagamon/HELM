use anyhow::Result;
use figment::{providers::Env, Figment};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // fields wired up across S2-S8
struct Settings {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_db_path")]
    db_path: String,
    #[serde(default = "default_log_buffer_size")]
    log_buffer_size: usize,
    #[serde(default = "default_log_lines_keep")]
    log_lines_keep: usize,
    #[serde(default = "default_retention_days")]
    output_logs_retention_days: i64,
    #[serde(default = "default_max_rows_total")]
    output_logs_max_rows_total: i64,
    #[serde(default = "default_max_rows_per_entity")]
    output_logs_max_rows_per_entity: i64,
    #[serde(default = "default_cleanup_interval")]
    output_logs_cleanup_interval_seconds: u64,
    #[serde(default = "default_vacuum_interval")]
    output_logs_vacuum_interval_seconds: u64,
    #[serde(default = "default_health_concurrency")]
    health_check_concurrency: usize,
    #[serde(default)]
    dashboard_pin: String,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    7010
}
fn default_db_path() -> String {
    "./data/dashboard.db".into()
}
fn default_log_buffer_size() -> usize {
    500
}
fn default_log_lines_keep() -> usize {
    1000
}
fn default_retention_days() -> i64 {
    14
}
fn default_max_rows_total() -> i64 {
    300_000
}
fn default_max_rows_per_entity() -> i64 {
    100_000
}
fn default_cleanup_interval() -> u64 {
    300
}
fn default_vacuum_interval() -> u64 {
    21_600
}
fn default_health_concurrency() -> usize {
    5
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "helm=info,helm_db=info,helm_api=info".into()),
        )
        .init();

    let settings: Settings = Figment::new().merge(Env::raw()).extract()?;

    let db = helm_db::Db::connect(&settings.db_path).await?;
    let log_buffer = helm_proc::LogBuffer::new(settings.log_buffer_size);
    let status = helm_proc::StatusBroadcaster::new(256);
    let pm = helm_proc::ProcessManager::new(db.clone(), log_buffer.clone(), status.clone());

    // Periodic services
    let metrics = helm_proc::MetricsCollector::new(
        pm.clone(),
        status.clone(),
        std::time::Duration::from_secs(5),
    );
    metrics.clone().spawn();

    let host = helm_proc::HostMonitor::new(std::time::Duration::from_secs(2));
    host.clone().spawn();

    let retention = helm_proc::LogRetention::new(
        db.clone(),
        helm_proc::RetentionConfig {
            retention_days: settings.output_logs_retention_days,
            max_rows_total: settings.output_logs_max_rows_total,
            max_rows_per_entity: settings.output_logs_max_rows_per_entity,
            cleanup_interval: std::time::Duration::from_secs(
                settings.output_logs_cleanup_interval_seconds,
            ),
            vacuum_interval: std::time::Duration::from_secs(
                settings.output_logs_vacuum_interval_seconds,
            ),
        },
    );
    retention.spawn();

    let health = helm_proc::HealthMonitor::new(
        db.clone(),
        status.clone(),
        settings.health_check_concurrency,
        std::time::Duration::from_secs(30),
    );
    health.spawn();

    let scheduler = helm_proc::Scheduler::new(pm.clone()).await?;
    if let Err(e) = scheduler.load_all(&db).await {
        tracing::warn!("scheduler: load_all failed: {e}");
    }

    let (restart_tx, mut restart_rx) = tokio::sync::watch::channel(false);
    let mut state = helm_api::make_state(db, pm, log_buffer, status, metrics, host, scheduler);
    state.dashboard_pin = settings.dashboard_pin.clone();
    state.restart_tx = Some(restart_tx);

    let cwd_static = std::path::PathBuf::from("static");
    let exe_static = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("static")));
    let static_dir = if cwd_static.exists() {
        cwd_static
    } else if let Some(p) = exe_static.filter(|p| p.exists()) {
        p
    } else {
        std::path::PathBuf::from("static")
    };

    let mut app = helm_api::build_router(state);
    if static_dir.exists() {
        let index = static_dir.join("index.html");
        app = app.fallback_service(
            ServeDir::new(&static_dir).fallback(ServeFile::new(index)),
        );
    }

    let addr: SocketAddr = format!("{}:{}", settings.host, settings.port).parse()?;
    info!("HELM (Rust) listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            loop {
                if *restart_rx.borrow() {
                    break;
                }
                if restart_rx.changed().await.is_err() {
                    break;
                }
            }
        })
        .await?;
    Ok(())
}
