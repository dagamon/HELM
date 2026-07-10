use crate::{
    error::ApiError,
    routers::services::{build_spawn_spec, map_spawn_error},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use helm_core::models::{StackCreate, StackResponse, StackUpdate};
use helm_db::repo::{de_json, services::ServiceRow, stacks};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/stacks", get(list).post(create))
        .route("/api/stacks/:id", get(get_one).put(update).delete(delete))
        .route("/api/stacks/:id/start", axum::routing::post(start))
        .route("/api/stacks/:id/stop", axum::routing::post(stop))
        .route("/api/stacks/:id/restart", axum::routing::post(restart))
}

fn is_running(state: &AppState, service_id: i64) -> bool {
    state.pm.get(&format!("service_{service_id}")).is_some()
}

fn counts(state: &AppState, members: &[ServiceRow]) -> (i64, i64) {
    let running = members.iter().filter(|r| is_running(state, r.id)).count() as i64;
    (members.len() as i64, running)
}

async fn stack_response(state: &AppState, row: stacks::StackRow) -> Result<StackResponse, ApiError> {
    let members = stacks::services_in_stack(&state.db.pool, row.id).await?;
    let (total, running) = counts(state, &members);
    Ok(row.into_response(total, running)?)
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<StackResponse>>, ApiError> {
    let rows = stacks::list(&state.db.pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(stack_response(&state, row).await?);
    }
    Ok(Json(out))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<StackResponse>, ApiError> {
    let row = stacks::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Stack {id} not found")))?;
    Ok(Json(stack_response(&state, row).await?))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<StackCreate>,
) -> Result<(axum::http::StatusCode, Json<StackResponse>), ApiError> {
    let id = stacks::create(&state.db.pool, &body).await?;
    let row = stacks::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("create succeeded but get failed".into()))?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(stack_response(&state, row).await?),
    ))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<StackUpdate>,
) -> Result<Json<StackResponse>, ApiError> {
    if stacks::get(&state.db.pool, id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Stack {id} not found")));
    }
    stacks::update(&state.db.pool, id, &body).await?;
    let row = stacks::get(&state.db.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("update succeeded but get failed".into()))?;
    Ok(Json(stack_response(&state, row).await?))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<axum::http::StatusCode, ApiError> {
    if stacks::get(&state.db.pool, id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Stack {id} not found")));
    }
    stacks::delete(&state.db.pool, id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Order stack members so that intra-stack `depends_on` targets come first
/// (Kahn's algorithm). Ties keep the incoming (sort_order) order; on a
/// dependency cycle the unresolved tail is appended in incoming order.
fn topo_order(members: Vec<ServiceRow>) -> Vec<ServiceRow> {
    let ids: HashSet<i64> = members.iter().map(|r| r.id).collect();
    let deps_of = |row: &ServiceRow| -> Vec<i64> {
        de_json::<Vec<i64>>(&row.depends_on)
            .ok()
            .flatten()
            .unwrap_or_default()
            .into_iter()
            .filter(|d| ids.contains(d))
            .collect()
    };

    let mut in_deg: HashMap<i64, usize> = HashMap::new();
    let mut dependents: HashMap<i64, Vec<i64>> = HashMap::new();
    for row in &members {
        let deps = deps_of(row);
        in_deg.insert(row.id, deps.len());
        for d in deps {
            dependents.entry(d).or_default().push(row.id);
        }
    }

    let mut queue: VecDeque<i64> = members
        .iter()
        .filter(|r| in_deg[&r.id] == 0)
        .map(|r| r.id)
        .collect();
    let mut order: Vec<i64> = Vec::with_capacity(members.len());
    while let Some(id) = queue.pop_front() {
        order.push(id);
        for &dep in dependents.get(&id).into_iter().flatten() {
            let deg = in_deg.get_mut(&dep).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(dep);
            }
        }
    }

    let placed: HashSet<i64> = order.iter().copied().collect();
    let mut by_id: HashMap<i64, ServiceRow> =
        members.into_iter().map(|r| (r.id, r)).collect();
    let mut out: Vec<ServiceRow> = order.into_iter().filter_map(|id| by_id.remove(&id)).collect();
    // Cycle fallback: whatever Kahn couldn't place, in natural order.
    let mut rest: Vec<ServiceRow> = by_id.into_values().filter(|r| !placed.contains(&r.id)).collect();
    rest.sort_by_key(|r| (r.sort_order, r.id));
    out.extend(rest);
    out
}

#[derive(Debug, Serialize)]
struct MemberOutcome {
    id: i64,
    name: String,
    outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct StackActionResult {
    status: String,
    services: Vec<MemberOutcome>,
}

async fn members_or_404(state: &AppState, id: i64) -> Result<Vec<ServiceRow>, ApiError> {
    if stacks::get(&state.db.pool, id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Stack {id} not found")));
    }
    Ok(stacks::services_in_stack(&state.db.pool, id).await?)
}

async fn start_members(state: &AppState, members: Vec<ServiceRow>) -> Vec<MemberOutcome> {
    let mut results = Vec::new();
    for row in topo_order(members) {
        if is_running(state, row.id) {
            results.push(MemberOutcome {
                id: row.id,
                name: row.name.clone(),
                outcome: "already_running".into(),
                error: None,
            });
            continue;
        }
        let outcome = match build_spawn_spec(&row).await {
            Ok(spec) => match state.pm.spawn(spec).await {
                Ok(_) => MemberOutcome {
                    id: row.id,
                    name: row.name.clone(),
                    outcome: "started".into(),
                    error: None,
                },
                Err(e) => MemberOutcome {
                    id: row.id,
                    name: row.name.clone(),
                    outcome: "failed".into(),
                    error: Some(map_spawn_error(e).to_string()),
                },
            },
            Err(e) => MemberOutcome {
                id: row.id,
                name: row.name.clone(),
                outcome: "failed".into(),
                error: Some(e.to_string()),
            },
        };
        results.push(outcome);
    }
    results
}

async fn stop_members(state: &AppState, members: Vec<ServiceRow>) -> Vec<MemberOutcome> {
    let mut ordered = topo_order(members);
    ordered.reverse(); // dependents stop before their dependencies
    let mut results = Vec::new();
    for row in ordered {
        if !is_running(state, row.id) {
            results.push(MemberOutcome {
                id: row.id,
                name: row.name.clone(),
                outcome: "already_stopped".into(),
                error: None,
            });
            continue;
        }
        let key = format!("service_{}", row.id);
        match state.pm.terminate(&key).await {
            Ok(()) => results.push(MemberOutcome {
                id: row.id,
                name: row.name.clone(),
                outcome: "stopped".into(),
                error: None,
            }),
            Err(e) => results.push(MemberOutcome {
                id: row.id,
                name: row.name.clone(),
                outcome: "failed".into(),
                error: Some(e.to_string()),
            }),
        }
    }
    results
}

fn summarize(results: Vec<MemberOutcome>) -> Json<StackActionResult> {
    let status = if results.iter().any(|r| r.outcome == "failed") {
        "partial"
    } else {
        "ok"
    };
    Json(StackActionResult {
        status: status.into(),
        services: results,
    })
}

async fn start(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<StackActionResult>, ApiError> {
    let members = members_or_404(&state, id).await?;
    Ok(summarize(start_members(&state, members).await))
}

async fn stop(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<StackActionResult>, ApiError> {
    let members = members_or_404(&state, id).await?;
    Ok(summarize(stop_members(&state, members).await))
}

async fn restart(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<StackActionResult>, ApiError> {
    let members = members_or_404(&state, id).await?;
    let mut results = stop_members(&state, members.clone()).await;
    results.extend(start_members(&state, members).await);
    Ok(summarize(results))
}
