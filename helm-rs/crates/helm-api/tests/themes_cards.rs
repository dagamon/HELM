//! Integration tests for the theme catalog endpoint and per-card panel colors.

use helm_api::{build_router, make_state};
use helm_db::Db;
use helm_proc::{LogBuffer, MetricsCollector, ProcessManager, Scheduler, StatusBroadcaster};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::net::TcpListener;

struct TestServer {
    addr: std::net::SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    _tmp: tempfile::TempDir,
}

impl TestServer {
    fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

async fn start_server() -> TestServer {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("themes.db");
    let db = Db::connect(db_path.to_str().unwrap()).await.unwrap();
    let log_buffer = LogBuffer::new(200);
    let status = StatusBroadcaster::new(64);
    let pm = ProcessManager::new(db.clone(), log_buffer.clone(), status.clone());
    let metrics = MetricsCollector::new(pm.clone(), status.clone(), Duration::from_secs(5));
    let host = helm_proc::HostMonitor::new(Duration::from_secs(2));
    let scheduler = Scheduler::new(pm.clone()).await.unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let state = make_state(db, pm, log_buffer, status, metrics, host, scheduler);
    let router = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    TestServer {
        addr,
        shutdown_tx: Some(shutdown_tx),
        _tmp: tmp,
    }
}

#[tokio::test]
async fn themes_endpoint_lists_bundled_presets() {
    let srv = start_server().await;
    let client = reqwest::Client::new();

    let resp = client.get(srv.url("/api/themes")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let themes: Vec<Value> = resp.json().await.unwrap();
    assert!(themes.len() >= 5, "expected bundled themes, got {}", themes.len());

    let names: Vec<&str> = themes
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    for expected in ["helm-dark", "abyss", "ember", "vapor", "daylight"] {
        assert!(names.contains(&expected), "missing theme {expected}");
    }
    for theme in &themes {
        assert!(theme.get("colors").map(Value::is_object).unwrap_or(false));
        assert!(
            theme.get("panels").map(Value::is_object).unwrap_or(false),
            "theme {:?} has no panels palette",
            theme.get("name")
        );
    }
}

#[tokio::test]
async fn service_card_color_roundtrip() {
    let srv = start_server().await;
    let client = reqwest::Client::new();

    let created: Value = client
        .post(srv.url("/api/services"))
        .json(&json!({"name": "svc", "type": "other", "command": "echo", "card_color": "ocean"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = created["id"].as_i64().unwrap();
    assert_eq!(created["card_color"], "ocean");

    let updated: Value = client
        .put(srv.url(&format!("/api/services/{id}")))
        .json(&json!({"card_color": "plum"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(updated["card_color"], "plum");

    // Empty string resets the panel back to the theme default.
    let reset: Value = client
        .put(srv.url(&format!("/api/services/{id}")))
        .json(&json!({"card_color": ""}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(reset["card_color"], "");
}

#[tokio::test]
async fn stack_card_color_roundtrip() {
    let srv = start_server().await;
    let client = reqwest::Client::new();

    let created: Value = client
        .post(srv.url("/api/stacks"))
        .json(&json!({"name": "stack", "card_color": "forest"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = created["id"].as_i64().unwrap();
    assert_eq!(created["card_color"], "forest");

    let updated: Value = client
        .put(srv.url(&format!("/api/stacks/{id}")))
        .json(&json!({"card_color": "amber"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(updated["card_color"], "amber");
}

#[tokio::test]
async fn theme_setting_stores_active_name() {
    let srv = start_server().await;
    let client = reqwest::Client::new();

    let put = client
        .put(srv.url("/api/settings/theme"))
        .json(&json!({"name": "abyss"}))
        .send()
        .await
        .unwrap();
    assert_eq!(put.status(), 200);

    let got: Value = client
        .get(srv.url("/api/settings/theme"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(got["name"], "abyss");
}
