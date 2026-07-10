//! Integration tests for the WebSocket endpoints (`/ws/status`, `/ws/logs/...`).
//!
//! Wires up the full AppState (DB, ProcessManager, LogBuffer, StatusBroadcaster),
//! serves the axum router on an ephemeral port, and exercises the WS upgrade
//! using tokio-tungstenite as the client.

use futures_util::{SinkExt, StreamExt};
use helm_api::{build_router, make_state};
use helm_db::Db;
use helm_platform::RunMode;
use helm_proc::{
    LogBuffer, MetricsCollector, ProcessManager, Scheduler, SpawnSpec, StatusBroadcaster,
};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::tungstenite::Message;

struct TestServer {
    addr: std::net::SocketAddr,
    pm: Arc<ProcessManager>,
    _shutdown: tokio::sync::oneshot::Sender<()>,
    _tmp: tempfile::TempDir,
    db: Db,
    status: Arc<StatusBroadcaster>,
}

async fn start_server() -> TestServer {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("ws.db");
    let db = Db::connect(db_path.to_str().unwrap()).await.unwrap();
    let log_buffer = LogBuffer::new(500);
    let status = StatusBroadcaster::new(256);
    let pm = ProcessManager::new(db.clone(), log_buffer.clone(), status.clone());
    let metrics = MetricsCollector::new(pm.clone(), status.clone(), Duration::from_secs(5));
    let host = helm_proc::HostMonitor::new(Duration::from_secs(2));
    let scheduler = Scheduler::new(pm.clone()).await.unwrap();

    let state = make_state(
        db.clone(),
        pm.clone(),
        log_buffer.clone(),
        status.clone(),
        metrics,
        host,
        scheduler,
    );
    let router = build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = rx.await;
            })
            .await
            .unwrap();
    });

    // Give the server a beat to bind
    tokio::time::sleep(Duration::from_millis(50)).await;

    TestServer {
        addr,
        pm,
        _shutdown: tx,
        _tmp: tmp,
        db,
        status,
    }
}

fn shell_spec(entity_type: &str, entity_id: i64, script: &str) -> SpawnSpec {
    SpawnSpec {
        entity_type: entity_type.into(),
        entity_id,
        command: script.into(),
        args: vec![],
        cwd: None,
        env: None,
        venv_path: None,
        run_mode: RunMode::Shell,
        restart_on_crash: false,
        webhook_url: None,
        name: "ws-test".into(),
        depends_on: vec![],
    }
}

#[cfg(windows)]
fn echo_script(text: &str) -> String {
    format!("echo {}", text)
}
#[cfg(unix)]
fn echo_script(text: &str) -> String {
    format!("echo {}", text)
}

#[tokio::test]
async fn ws_status_receives_running_event() {
    let srv = start_server().await;

    let url = format!("ws://{}/ws/status", srv.addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    // Spawn a short-lived process — should emit `running` then a terminal status.
    let spec = shell_spec("script", 1, &echo_script("hi"));
    let _ = srv.pm.spawn(spec).await.unwrap();

    let msg = timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("ws timeout")
        .expect("ws closed")
        .expect("ws error");
    let text = match msg {
        Message::Text(t) => t,
        other => panic!("unexpected: {other:?}"),
    };
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["entity_type"], "script");
    assert_eq!(v["entity_id"], 1);
    assert_eq!(v["status"], "running");
    assert!(v["pid"].is_u64());

    let _ = ws.send(Message::Close(None)).await;
}

#[tokio::test]
async fn ws_logs_delivers_history_and_live_lines() {
    let srv = start_server().await;

    // Spawn and let it complete so history populates.
    let spec = shell_spec("script", 2, &echo_script("hello-from-history"));
    let mp = srv.pm.spawn(spec).await.unwrap();

    // Wait for completion in DB.
    for _ in 0..100 {
        if let Some(row) = helm_db::repo::run_logs::get(&srv.db.pool, mp.run_log_id)
            .await
            .unwrap()
        {
            if row.status.as_deref() == Some("success") {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connect AFTER process exited — history should still replay buffered lines.
    let url = format!("ws://{}/ws/logs/script/2", srv.addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    let msg = timeout(Duration::from_secs(3), ws.next())
        .await
        .expect("ws timeout for history")
        .expect("ws closed")
        .expect("ws error");
    let text = match msg {
        Message::Text(t) => t,
        other => panic!("unexpected: {other:?}"),
    };
    let v: Value = serde_json::from_str(&text).unwrap();
    let line = v["text"].as_str().unwrap_or("").trim_end().to_string();
    assert_eq!(line, "hello-from-history");
    assert_eq!(v["stream"], "stdout");

    let _ = ws.send(Message::Close(None)).await;
}

#[tokio::test]
async fn ws_status_forwards_direct_broadcast() {
    use helm_proc::StatusEvent;
    let srv = start_server().await;

    let url = format!("ws://{}/ws/status", srv.addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    // Push a synthetic event straight through the broadcaster.
    srv.status
        .send(StatusEvent::new("service", 42, "stopped", None));

    let msg = timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("ws timeout")
        .expect("ws closed")
        .expect("ws error");
    let text = match msg {
        Message::Text(t) => t,
        other => panic!("unexpected: {other:?}"),
    };
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["entity_type"], "service");
    assert_eq!(v["entity_id"], 42);
    assert_eq!(v["status"], "stopped");

    let _ = ws.send(Message::Close(None)).await;
}
