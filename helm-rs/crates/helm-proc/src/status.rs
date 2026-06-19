//! Status fan-out for WebSocket subscribers (mirrors `StatusBroadcaster`).

use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct StatusEvent {
    pub entity_type: String,
    pub entity_id: i64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<serde_json::Value>,
}

impl StatusEvent {
    pub fn new(entity_type: &str, entity_id: i64, status: &str, pid: Option<u32>) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id,
            status: status.into(),
            pid,
            metrics: None,
        }
    }
}

pub struct StatusBroadcaster {
    tx: broadcast::Sender<StatusEvent>,
}

impl StatusBroadcaster {
    pub fn new(capacity: usize) -> Arc<Self> {
        let (tx, _) = broadcast::channel(capacity.max(1));
        Arc::new(Self { tx })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StatusEvent> {
        self.tx.subscribe()
    }

    /// Best-effort send; ignores "no active receivers" — that is expected when no
    /// clients are connected.
    pub fn send(&self, event: StatusEvent) {
        let _ = self.tx.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subscriber_receives_event() {
        let b = StatusBroadcaster::new(8);
        let mut rx = b.subscribe();
        b.send(StatusEvent::new("service", 1, "running", Some(123)));
        let ev = rx.recv().await.unwrap();
        assert_eq!(ev.status, "running");
        assert_eq!(ev.pid, Some(123));
    }

    #[tokio::test]
    async fn send_with_no_receivers_is_ok() {
        let b = StatusBroadcaster::new(8);
        // does not panic
        b.send(StatusEvent::new("service", 1, "stopped", None));
    }
}
