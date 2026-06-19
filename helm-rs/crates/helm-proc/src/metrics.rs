//! Periodic CPU/RSS metrics collection for managed processes.
//! Port of `server/services/metrics_collector.py`. Uses `sysinfo` instead of psutil.

use crate::{
    process::ProcessManager,
    status::{StatusBroadcaster, StatusEvent},
};
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::Mutex;
use serde::Serialize;
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};
use sysinfo::{Pid, System};
use tokio::task::JoinHandle;
use tracing::warn;

const MAX_SNAPSHOTS: usize = 60;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct MetricsSnapshot {
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub ts_secs: i64,
}

pub struct MetricsCollector {
    pm: Arc<ProcessManager>,
    status: Arc<StatusBroadcaster>,
    interval: Duration,
    snapshots: DashMap<String, Mutex<VecDeque<MetricsSnapshot>>>,
}

impl MetricsCollector {
    pub fn new(
        pm: Arc<ProcessManager>,
        status: Arc<StatusBroadcaster>,
        interval: Duration,
    ) -> Arc<Self> {
        Arc::new(Self {
            pm,
            status,
            interval,
            snapshots: DashMap::new(),
        })
    }

    pub fn snapshots(&self, key: &str) -> Vec<MetricsSnapshot> {
        self.snapshots
            .get(key)
            .map(|m| m.lock().iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn spawn(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            // sysinfo requires two refresh cycles to produce meaningful CPU %.
            // We refresh per cycle and rely on cached state for the second sample.
            let mut sys = System::new();
            loop {
                if let Err(e) = self.collect(&mut sys).await {
                    warn!("metrics collect failed: {e}");
                }
                tokio::time::sleep(self.interval).await;
            }
        })
    }

    async fn collect(&self, sys: &mut System) -> anyhow::Result<()> {
        let procs = self.pm.running_processes();
        if procs.is_empty() {
            return Ok(());
        }

        sys.refresh_processes();

        // Clean stale snapshot keys
        let active: HashSet<String> = procs.iter().map(|p| p.key.clone()).collect();
        self.snapshots.retain(|k, _| active.contains(k));

        for mp in &procs {
            let descendants = collect_tree(sys, mp.pid);
            if descendants.is_empty() {
                continue;
            }
            let mut cpu = 0.0f32;
            let mut mem_bytes: u64 = 0;
            for pid in &descendants {
                if let Some(p) = sys.process(*pid) {
                    cpu += p.cpu_usage();
                    mem_bytes += p.memory();
                }
            }
            let snap = MetricsSnapshot {
                cpu_percent: (cpu * 10.0).round() / 10.0,
                memory_mb: ((mem_bytes as f32) / (1024.0 * 1024.0) * 10.0).round() / 10.0,
                ts_secs: Utc::now().timestamp(),
            };

            let entry = self
                .snapshots
                .entry(mp.key.clone())
                .or_insert_with(|| Mutex::new(VecDeque::with_capacity(MAX_SNAPSHOTS)));
            {
                let mut q = entry.lock();
                if q.len() == MAX_SNAPSHOTS {
                    q.pop_front();
                }
                q.push_back(snap);
            }

            self.status.send(StatusEvent {
                entity_type: mp.entity_type.clone(),
                entity_id: mp.entity_id,
                status: "running".into(),
                pid: Some(mp.pid),
                metrics: Some(serde_json::json!({
                    "cpu_percent": snap.cpu_percent,
                    "memory_mb": snap.memory_mb,
                })),
            });
        }
        Ok(())
    }
}

fn collect_tree(sys: &System, root_pid: u32) -> Vec<Pid> {
    let root = Pid::from_u32(root_pid);
    if sys.process(root).is_none() {
        return vec![];
    }
    let mut out = vec![root];
    let mut frontier = vec![root];
    while let Some(p) = frontier.pop() {
        for (pid, proc) in sys.processes() {
            if proc.parent() == Some(p) {
                out.push(*pid);
                frontier.push(*pid);
            }
        }
    }
    out
}
