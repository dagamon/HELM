//! In-memory ring buffer of recent log lines, keyed by `"{entity_type}_{entity_id}"`.
//!
//! Mirrors `server/services/log_buffer.py`. Uses `parking_lot::Mutex` because each
//! lock is held briefly and never spans an `.await`.

use dashmap::DashMap;
use parking_lot::Mutex;
use serde::Serialize;
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub stream: String,
    pub text: String,
    pub ts: String,
}

pub struct LogBuffer {
    buffers: DashMap<String, Mutex<VecDeque<LogEntry>>>,
    max: usize,
}

impl LogBuffer {
    pub fn new(max: usize) -> Arc<Self> {
        Arc::new(Self {
            buffers: DashMap::new(),
            max: max.max(1),
        })
    }

    pub fn append(&self, key: &str, entry: LogEntry) {
        let slot = self
            .buffers
            .entry(key.to_string())
            .or_insert_with(|| Mutex::new(VecDeque::with_capacity(self.max)));
        let mut q = slot.lock();
        if q.len() == self.max {
            q.pop_front();
        }
        q.push_back(entry);
    }

    pub fn recent(&self, key: &str, limit: usize) -> Vec<LogEntry> {
        let Some(slot) = self.buffers.get(key) else {
            return Vec::new();
        };
        let q = slot.lock();
        let start = q.len().saturating_sub(limit);
        q.iter().skip(start).cloned().collect()
    }

    pub fn clear(&self, key: &str) {
        self.buffers.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(text: &str) -> LogEntry {
        LogEntry {
            stream: "stdout".into(),
            text: text.into(),
            ts: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn append_and_recent_returns_chronological() {
        let lb = LogBuffer::new(10);
        lb.append("k", entry("a"));
        lb.append("k", entry("b"));
        let r = lb.recent("k", 10);
        assert_eq!(
            r.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn ring_drops_oldest() {
        let lb = LogBuffer::new(2);
        lb.append("k", entry("a"));
        lb.append("k", entry("b"));
        lb.append("k", entry("c"));
        let r = lb.recent("k", 10);
        assert_eq!(
            r.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(),
            ["b", "c"]
        );
    }

    #[test]
    fn recent_respects_limit() {
        let lb = LogBuffer::new(10);
        for c in ["a", "b", "c", "d"] {
            lb.append("k", entry(c));
        }
        let r = lb.recent("k", 2);
        assert_eq!(
            r.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(),
            ["c", "d"]
        );
    }

    #[test]
    fn recent_unknown_key_is_empty() {
        let lb = LogBuffer::new(10);
        assert!(lb.recent("nope", 10).is_empty());
    }

    #[test]
    fn clear_removes_key() {
        let lb = LogBuffer::new(10);
        lb.append("k", entry("a"));
        lb.clear("k");
        assert!(lb.recent("k", 10).is_empty());
    }
}
