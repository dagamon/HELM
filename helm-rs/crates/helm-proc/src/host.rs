//! Host-wide resource metrics for the diagnostics dashboard.
//! A background task keeps a fresh `HostSnapshot` so the HTTP handler can serve
//! it instantly without paying the two-refresh CPU sampling cost per request.

use parking_lot::RwLock;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use sysinfo::{Disks, System};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize, Default)]
pub struct CpuCore {
    pub name: String,
    pub usage: f32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub fs: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct HostSnapshot {
    pub cpu_brand: String,
    pub cpu_usage: f32,
    pub core_count: usize,
    pub cores: Vec<CpuCore>,
    pub mem_total_bytes: u64,
    pub mem_used_bytes: u64,
    pub mem_available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub uptime_seconds: u64,
    pub load_avg: [f64; 3],
    pub disks: Vec<DiskInfo>,
    pub ts_secs: i64,
}

pub struct HostMonitor {
    interval: Duration,
    latest: Arc<RwLock<HostSnapshot>>,
}

impl HostMonitor {
    pub fn new(interval: Duration) -> Arc<Self> {
        Arc::new(Self {
            interval,
            latest: Arc::new(RwLock::new(HostSnapshot::default())),
        })
    }

    pub fn snapshot(&self) -> HostSnapshot {
        self.latest.read().clone()
    }

    pub fn spawn(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut sys = System::new();
            // Prime CPU counters; the first sample after a single refresh is 0%.
            sys.refresh_cpu();
            sys.refresh_memory();
            tokio::time::sleep(Duration::from_millis(250)).await;
            loop {
                sys.refresh_cpu();
                sys.refresh_memory();
                *self.latest.write() = build_snapshot(&sys);
                tokio::time::sleep(self.interval).await;
            }
        })
    }
}

fn build_snapshot(sys: &System) -> HostSnapshot {
    let global = sys.global_cpu_info();
    let cores = sys
        .cpus()
        .iter()
        .map(|c| CpuCore {
            name: c.name().to_string(),
            usage: (c.cpu_usage() * 10.0).round() / 10.0,
        })
        .collect::<Vec<_>>();

    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|d| DiskInfo {
            name: d.name().to_string_lossy().into_owned(),
            mount: d.mount_point().to_string_lossy().into_owned(),
            fs: d.file_system().to_string_lossy().into_owned(),
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
        })
        .collect::<Vec<_>>();

    let load = System::load_average();

    HostSnapshot {
        cpu_brand: global.brand().trim().to_string(),
        cpu_usage: (global.cpu_usage() * 10.0).round() / 10.0,
        core_count: sys.cpus().len(),
        cores,
        mem_total_bytes: sys.total_memory(),
        mem_used_bytes: sys.used_memory(),
        mem_available_bytes: sys.available_memory(),
        swap_total_bytes: sys.total_swap(),
        swap_used_bytes: sys.used_swap(),
        uptime_seconds: System::uptime(),
        load_avg: [load.one, load.five, load.fifteen],
        disks,
        ts_secs: chrono::Utc::now().timestamp(),
    }
}
