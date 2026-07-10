//! Integration tests for the Rust runtime path (S9).
//!
//! `binary_path` flow is tested with a stand-in shell executable so the test is
//! fast and does not require `cargo` on PATH. `prebuild` flow is exercised with
//! an intentionally-broken manifest_path to assert error mapping.
//!
//! End-to-end compilation of `tests/fixtures/rust-app/` is documented in the
//! S9 report as a manual smoke step; running `cargo build` in CI per test run
//! would dominate test time.

use helm_api::{build_router, make_state};
use helm_core::models::ServiceCreate;
use helm_db::{repo, Db};
use helm_proc::{LogBuffer, MetricsCollector, ProcessManager, Scheduler, StatusBroadcaster};
use std::time::Duration;
use tokio::net::TcpListener;

async fn start_server() -> (std::net::SocketAddr, Db, tokio::sync::oneshot::Sender<()>) {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("rust.db");
    let db = Db::connect(db_path.to_str().unwrap()).await.unwrap();
    let log_buffer = LogBuffer::new(500);
    let status = StatusBroadcaster::new(64);
    let pm = ProcessManager::new(db.clone(), log_buffer.clone(), status.clone());
    let metrics = MetricsCollector::new(pm.clone(), status.clone(), Duration::from_secs(5));
    let host = helm_proc::HostMonitor::new(Duration::from_secs(2));
    let scheduler = Scheduler::new(pm.clone()).await.unwrap();

    let state = make_state(db.clone(), pm, log_buffer, status, metrics, host, scheduler);
    let router = build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let db_clone = db.clone();
    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = rx.await;
            })
            .await
            .unwrap();
        drop(db_clone); // keep tmp alive in this task scope
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    std::mem::forget(tmp);
    (addr, db, tx)
}

fn shell_bin_path() -> String {
    if cfg!(windows) {
        r"C:\Windows\System32\cmd.exe".into()
    } else {
        "/bin/sh".into()
    }
}

fn make_rust_service_create(
    name: &str,
    binary_path: Option<String>,
    manifest_path: Option<String>,
    prebuild: bool,
) -> ServiceCreate {
    ServiceCreate {
        name: name.into(),
        description: None,
        r#type: "rust".into(),
        command: None,
        cwd: None,
        venv_path: None,
        args: None,
        env: None,
        url: None,
        health_check_url: None,
        health_check_interval: 30,
        auto_start: false,
        restart_on_crash: false,
        platform: "all".into(),
        tags: None,
        depends_on: None,
        webhook_url: None,
        manifest_path,
        binary_path,
        cargo_profile: Some("release".into()),
        cargo_features: None,
        prebuild,
        stack_id: None,
        card_color: None,
    }
}

#[tokio::test]
async fn rust_service_binary_path_flow_spawns_and_completes() {
    let (addr, db, _shutdown) = start_server().await;

    let svc = make_rust_service_create("rust-bin", Some(shell_bin_path()), None, false);
    let id = repo::services::create(&db.pool, &svc).await.unwrap();

    // Override args: tell shell to print + exit. Done via PUT.
    let client = reqwest::Client::new();
    let args = if cfg!(windows) {
        serde_json::json!(["/c", "echo from-rust-binary"])
    } else {
        serde_json::json!(["-c", "echo from-rust-binary"])
    };
    let resp = client
        .put(format!("http://{}/api/services/{}", addr, id))
        .json(&serde_json::json!({"args": args}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client
        .post(format!("http://{}/api/services/{}/start", addr, id))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "start failed: {}",
        resp.text().await.unwrap()
    );

    // Wait for success.
    for _ in 0..100 {
        let rows = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT status FROM run_logs WHERE entity_type='service' AND entity_id=? ORDER BY id DESC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&db.pool)
        .await
        .unwrap();
        if let Some((Some(s),)) = rows {
            if s == "success" || s == "crashed" {
                assert_eq!(s, "success", "expected success");
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("run_logs never reached terminal status");
}

#[tokio::test]
async fn rust_service_prebuild_with_bad_manifest_returns_400() {
    let (addr, db, _shutdown) = start_server().await;

    let svc = make_rust_service_create(
        "rust-bad",
        None,
        Some("/this/manifest/does/not/exist/Cargo.toml".into()),
        true,
    );
    let id = repo::services::create(&db.pool, &svc).await.unwrap();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/api/services/{}/start", addr, id))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    // Either cargo missing (`cargo` not found in PATH) or cargo present + manifest invalid.
    assert!(
        status.is_client_error(),
        "expected 4xx, got {status}: {body}"
    );
}
