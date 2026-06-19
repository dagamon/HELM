//! Process management, log buffering, status broadcasting, and periodic
//! services (health, metrics, retention, cron scheduling).

pub mod health;
pub mod job;
pub mod log_buffer;
pub mod metrics;
pub mod process;
pub mod retention;
pub mod scheduler;
pub mod status;

pub use health::HealthMonitor;
pub use log_buffer::{LogBuffer, LogEntry};
pub use metrics::{MetricsCollector, MetricsSnapshot};
pub use process::{LogMsg, ManagedProcess, ProcessManager, SpawnSpec};
pub use retention::{LogRetention, RetentionConfig};
pub use scheduler::Scheduler;
pub use status::{StatusBroadcaster, StatusEvent};
