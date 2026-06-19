//! HTTP health checks for services with `health_check_url`.
//! Port of `server/services/health_monitor.py`.

use crate::status::{StatusBroadcaster, StatusEvent};
use helm_db::Db;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Semaphore, task::JoinHandle};
use tracing::warn;

pub struct HealthMonitor {
    db: Db,
    status: Arc<StatusBroadcaster>,
    http: reqwest::Client,
    semaphore: Arc<Semaphore>,
    interval: Duration,
}

impl HealthMonitor {
    pub fn new(
        db: Db,
        status: Arc<StatusBroadcaster>,
        concurrency: usize,
        interval: Duration,
    ) -> Self {
        Self {
            db,
            status,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest"),
            semaphore: Arc::new(Semaphore::new(concurrency.max(1))),
            interval,
        }
    }

    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.check_all().await {
                    warn!("health_monitor: cycle failed: {e}");
                }
                tokio::time::sleep(self.interval).await;
            }
        })
    }

    async fn check_all(&self) -> anyhow::Result<()> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: i64,
            health_check_url: Option<String>,
        }
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, health_check_url FROM services
             WHERE health_check_url IS NOT NULL AND health_check_url != ''",
        )
        .fetch_all(&self.db.pool)
        .await?;

        let mut handles = Vec::with_capacity(rows.len());
        for row in rows {
            let url = match row.health_check_url {
                Some(u) if !u.is_empty() => u,
                _ => continue,
            };
            let sem = self.semaphore.clone();
            let http = self.http.clone();
            let status = self.status.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok();
                let healthy = match http.get(&url).send().await {
                    Ok(resp) => resp.status().as_u16() < 400,
                    Err(_) => false,
                };
                let s = if healthy { "healthy" } else { "unhealthy" };
                status.send(StatusEvent::new("service", row.id, s, None));
            }));
        }
        for h in handles {
            let _ = h.await;
        }
        Ok(())
    }
}
