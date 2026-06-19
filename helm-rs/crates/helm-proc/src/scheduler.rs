//! Cron scheduler for scripts with `cron_enabled=1`.
//! Port of `server/services/scheduler.py` using `tokio-cron-scheduler`.

use crate::process::{ProcessManager, SpawnSpec};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use helm_db::{repo::de_json, Db};
use helm_platform::{is_compatible, RunMode};
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct CronScriptRow {
    id: i64,
    name: String,
    command: String,
    run_mode: Option<String>,
    cwd: Option<String>,
    args: Option<String>,
    platform: String,
    cron_schedule: Option<String>,
}

pub struct Scheduler {
    inner: JobScheduler,
    pm: Arc<ProcessManager>,
    jobs: DashMap<i64, Uuid>,
}

impl Scheduler {
    pub async fn new(pm: Arc<ProcessManager>) -> anyhow::Result<Arc<Self>> {
        let inner = JobScheduler::new().await?;
        inner.start().await?;
        Ok(Arc::new(Self {
            inner,
            pm,
            jobs: DashMap::new(),
        }))
    }

    /// Load all `scripts WHERE cron_enabled=1 AND cron_schedule IS NOT NULL`
    /// and register a job per row.
    pub async fn load_all(self: &Arc<Self>, db: &Db) -> anyhow::Result<usize> {
        let rows: Vec<CronScriptRow> = sqlx::query_as(
            "SELECT id, name, command, run_mode, cwd, args, platform, cron_schedule
             FROM scripts
             WHERE cron_enabled = 1 AND cron_schedule IS NOT NULL",
        )
        .fetch_all(&db.pool)
        .await?;
        let mut added = 0usize;
        for row in rows {
            if let Err(e) = self.add(row).await {
                warn!("scheduler: failed to add script: {e}");
            } else {
                added += 1;
            }
        }
        info!("scheduler: loaded {added} cron job(s)");
        Ok(added)
    }

    /// Validate a 5-field crontab expression (`min hour dom mon dow`).
    pub fn validate_cron(schedule: &str) -> bool {
        Job::new_async(
            prepend_seconds(schedule).as_str(),
            |_, _| Box::pin(async {}),
        )
        .is_ok()
    }

    pub async fn remove(&self, script_id: i64) -> anyhow::Result<()> {
        if let Some((_, uuid)) = self.jobs.remove(&script_id) {
            let sched = self.inner.clone();
            sched.remove(&uuid).await?;
        }
        Ok(())
    }

    pub async fn next_run(&self, script_id: i64) -> Option<DateTime<Utc>> {
        let uuid = self.jobs.get(&script_id).map(|v| *v)?;
        let mut sched = self.inner.clone();
        sched.next_tick_for_job(uuid).await.ok().flatten()
    }

    /// Snapshot of every scheduled job: script_id → next-fire UTC timestamp.
    pub async fn all_next_runs(&self) -> Vec<(i64, Option<DateTime<Utc>>)> {
        let mut out = Vec::with_capacity(self.jobs.len());
        let mut sched = self.inner.clone();
        for entry in self.jobs.iter() {
            let id = *entry.key();
            let uuid = *entry.value();
            let next = sched.next_tick_for_job(uuid).await.ok().flatten();
            out.push((id, next));
        }
        out
    }

    async fn add(self: &Arc<Self>, row: CronScriptRow) -> anyhow::Result<()> {
        if !is_compatible(&row.platform) {
            info!("scheduler: skip script {} — incompatible platform", row.id);
            return Ok(());
        }
        let cron = row
            .cron_schedule
            .clone()
            .ok_or_else(|| anyhow::anyhow!("cron_schedule is None"))?;
        let schedule = prepend_seconds(&cron);

        // Drop existing job for this script before registering replacement.
        self.remove(row.id).await.ok();

        let pm = self.pm.clone();
        let script_id = row.id;
        let name = row.name.clone();
        let command = row.command.clone();
        let cwd = row.cwd.clone();
        let args: Vec<String> = de_json::<Vec<String>>(&row.args)?.unwrap_or_default();
        let run_mode = match row.run_mode.as_deref() {
            Some("shell") => RunMode::Shell,
            _ => RunMode::Exec,
        };

        let job = Job::new_async(schedule.as_str(), move |_uuid, _l| {
            let pm = pm.clone();
            let name = name.clone();
            let command = command.clone();
            let cwd = cwd.clone();
            let args = args.clone();
            Box::pin(async move {
                let spec = SpawnSpec {
                    entity_type: "script".into(),
                    entity_id: script_id,
                    command,
                    args,
                    cwd,
                    env: None,
                    venv_path: None,
                    run_mode,
                    restart_on_crash: false,
                    webhook_url: None,
                    name,
                    depends_on: vec![],
                };
                if let Err(e) = pm.spawn(spec).await {
                    let msg = e.to_string();
                    if msg.contains("already running") {
                        info!("scheduler: skipping script {script_id} — previous run active");
                    } else {
                        warn!("scheduler: spawn for script {script_id} failed: {e}");
                    }
                }
            })
        })?;
        let sched = self.inner.clone();
        let uuid = sched.add(job).await?;
        self.jobs.insert(row.id, uuid);
        Ok(())
    }
}

/// Convert a Python-style 5-field crontab (`min hour dom mon dow`) to the
/// 6-field form (`sec min hour dom mon dow`) that tokio-cron-scheduler expects.
fn prepend_seconds(schedule: &str) -> String {
    let fields = schedule.split_whitespace().count();
    if fields == 5 {
        format!("0 {schedule}")
    } else {
        schedule.to_string()
    }
}
