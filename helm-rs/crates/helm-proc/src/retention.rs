//! Periodic pruning of the `output_logs` table to keep SQLite bounded.
//! Mirrors `server/services/log_retention.py`.

use helm_db::{repo::output_logs, Db};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy)]
pub struct RetentionConfig {
    pub retention_days: i64,
    pub max_rows_total: i64,
    pub max_rows_per_entity: i64,
    pub cleanup_interval: Duration,
    pub vacuum_interval: Duration,
}

pub struct LogRetention {
    db: Db,
    cfg: RetentionConfig,
    last_vacuum: Option<Instant>,
}

impl LogRetention {
    pub fn new(db: Db, cfg: RetentionConfig) -> Self {
        Self {
            db,
            cfg,
            last_vacuum: None,
        }
    }

    /// Spawn the retention loop. Returns the handle so callers can `abort()`
    /// on shutdown.
    pub fn spawn(self) -> JoinHandle<()> {
        let mut me = self;
        tokio::spawn(async move {
            loop {
                if let Err(e) = me.run_once().await {
                    warn!("log_retention: cleanup failed: {e}");
                }
                tokio::time::sleep(me.cfg.cleanup_interval).await;
            }
        })
    }

    pub async fn run_once(&mut self) -> anyhow::Result<()> {
        let stats = output_logs::cleanup(
            &self.db.pool,
            self.cfg.retention_days,
            self.cfg.max_rows_total,
            self.cfg.max_rows_per_entity,
        )
        .await?;
        if stats.deleted_total > 0 {
            info!(
                age = stats.deleted_by_age,
                total_cap = stats.deleted_by_total_cap,
                entity_cap = stats.deleted_by_entity_cap,
                remaining = stats.remaining_rows,
                "log_retention.cleanup_completed",
            );
        }
        self.maybe_vacuum(stats.deleted_total).await?;
        Ok(())
    }

    async fn maybe_vacuum(&mut self, deleted: i64) -> anyhow::Result<()> {
        if self.cfg.vacuum_interval.is_zero() || deleted <= 0 {
            return Ok(());
        }
        let due = match self.last_vacuum {
            Some(t) => t.elapsed() >= self.cfg.vacuum_interval,
            None => true,
        };
        if !due {
            return Ok(());
        }
        output_logs::vacuum(&self.db.pool).await?;
        self.last_vacuum = Some(Instant::now());
        info!("log_retention.vacuum_completed");
        Ok(())
    }
}
