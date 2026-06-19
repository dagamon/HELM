//! WebSocket endpoints — port of `server/routers/ws.py`.
//!
//! `/ws/logs/{entity_type}/{entity_id}` — sends buffered history then forwards
//! every new line emitted by [`helm_proc::ProcessManager`].
//!
//! `/ws/status` — forwards [`helm_proc::StatusEvent`]s for every entity.

use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use helm_proc::{LogMsg, StatusEvent};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;
use tracing::debug;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ws/logs/:entity_type/:entity_id", get(ws_logs))
        .route("/ws/status", get(ws_status))
}

async fn ws_logs(
    ws: WebSocketUpgrade,
    Path((entity_type, entity_id)): Path<(String, i64)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_logs(socket, state, entity_type, entity_id))
}

async fn handle_logs(mut socket: WebSocket, state: AppState, entity_type: String, entity_id: i64) {
    let key = format!("{entity_type}_{entity_id}");

    // Subscribe BEFORE sending history to avoid missing lines emitted while we
    // were dumping the buffer.
    let mut rx = state.pm.subscribe_logs(&key);

    for entry in state.log_buffer.recent(&key, 200) {
        let msg = LogMsg {
            stream: entry.stream,
            text: entry.text,
            ts: entry.ts,
        };
        if !send_json(&mut socket, &msg).await {
            return;
        }
    }

    loop {
        tokio::select! {
            biased;
            client_msg = socket.recv() => {
                match client_msg {
                    None | Some(Err(_)) => break,
                    Some(Ok(Message::Close(_))) => break,
                    _ => {}
                }
            }
            msg = rx.recv() => match msg {
                Ok(m) => {
                    if !send_json(&mut socket, &m).await {
                        break;
                    }
                }
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => {
                    debug!("ws_logs {key}: lagged {n} messages");
                }
            },
        }
    }
}

async fn ws_status(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_status(socket, state.status.clone()))
}

async fn handle_status(mut socket: WebSocket, status: Arc<helm_proc::StatusBroadcaster>) {
    let mut rx = status.subscribe();
    loop {
        tokio::select! {
            biased;
            client_msg = socket.recv() => {
                match client_msg {
                    None | Some(Err(_)) => break,
                    Some(Ok(Message::Close(_))) => break,
                    _ => {}
                }
            }
            ev = rx.recv() => match ev {
                Ok(ev) => {
                    if !send_json::<StatusEvent>(&mut socket, &ev).await {
                        break;
                    }
                }
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => {
                    debug!("ws_status: lagged {n} events");
                }
            },
        }
    }
}

async fn send_json<T: Serialize>(socket: &mut WebSocket, value: &T) -> bool {
    match serde_json::to_string(value) {
        Ok(s) => socket.send(Message::Text(s)).await.is_ok(),
        Err(e) => {
            debug!("ws serialize error: {e}");
            true
        }
    }
}
