//! Host + HELM-process diagnostics dashboard data.
//! `GET /api/system/diagnostics` returns the latest host snapshot plus a
//! per-managed-process resource breakdown, so the UI can show how much of the
//! machine HELM's own services account for.

use crate::{error::ApiError, state::AppState};
use axum::{extract::State, routing::get, Json, Router};
use helm_proc::HostSnapshot;
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/system/diagnostics", get(diagnostics))
}

#[derive(Serialize)]
pub struct ProcDiag {
    pub entity_type: String,
    pub entity_id: i64,
    pub name: String,
    pub pid: u32,
    pub cpu_percent: f32,
    pub memory_mb: f32,
}

#[derive(Serialize)]
pub struct HelmDiag {
    pub uptime_seconds: u64,
    pub service_count: i64,
    pub running_count: usize,
    pub cpu_percent: f32,
    pub memory_mb: f32,
}

#[derive(Serialize)]
pub struct Diagnostics {
    pub host: HostSnapshot,
    pub helm: HelmDiag,
    pub processes: Vec<ProcDiag>,
}

pub async fn diagnostics(State(state): State<AppState>) -> Result<Json<Diagnostics>, ApiError> {
    let host = state.host.snapshot();
    let running = state.pm.running_processes();

    let mut processes = Vec::with_capacity(running.len());
    let mut total_cpu = 0.0f32;
    let mut total_mem = 0.0f32;
    for mp in &running {
        let (cpu, mem) = state
            .metrics
            .snapshots(&mp.key)
            .last()
            .map(|s| (s.cpu_percent, s.memory_mb))
            .unwrap_or((0.0, 0.0));
        total_cpu += cpu;
        total_mem += mem;
        processes.push(ProcDiag {
            entity_type: mp.entity_type.clone(),
            entity_id: mp.entity_id,
            name: mp.name.clone(),
            pid: mp.pid,
            cpu_percent: cpu,
            memory_mb: mem,
        });
    }

    let service_count = state.db.service_count().await.unwrap_or(0);

    Ok(Json(Diagnostics {
        host,
        helm: HelmDiag {
            uptime_seconds: state.started_at.elapsed().as_secs(),
            service_count,
            running_count: running.len(),
            cpu_percent: (total_cpu * 10.0).round() / 10.0,
            memory_mb: (total_mem * 10.0).round() / 10.0,
        },
        processes,
    }))
}
