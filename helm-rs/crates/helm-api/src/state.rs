use helm_proc::{
    HostMonitor, LogBuffer, MetricsCollector, ProcessManager, Scheduler, StatusBroadcaster,
};
use std::{sync::Arc, time::Instant};
use tokio::sync::watch;

#[derive(Clone)]
pub struct AppState {
    pub db: helm_db::Db,
    pub started_at: Arc<Instant>,
    pub pm: Arc<ProcessManager>,
    pub log_buffer: Arc<LogBuffer>,
    pub status: Arc<StatusBroadcaster>,
    pub metrics: Arc<MetricsCollector>,
    pub host: Arc<HostMonitor>,
    pub scheduler: Arc<Scheduler>,
    pub dashboard_pin: String,
    pub restart_tx: Option<watch::Sender<bool>>,
}
