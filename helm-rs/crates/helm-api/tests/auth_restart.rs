use helm_api::{build_router, make_state};
use helm_db::Db;
use helm_proc::{LogBuffer, MetricsCollector, ProcessManager, Scheduler, StatusBroadcaster};
use reqwest::header::SET_COOKIE;
use std::time::Duration;
use tokio::net::TcpListener;

struct TestServer {
    addr: std::net::SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    _tmp: tempfile::TempDir,
}

impl TestServer {
    fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

async fn start_server(pin: &str, restartable: bool) -> TestServer {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("auth.db");
    let db = Db::connect(db_path.to_str().unwrap()).await.unwrap();
    let log_buffer = LogBuffer::new(200);
    let status = StatusBroadcaster::new(64);
    let pm = ProcessManager::new(db.clone(), log_buffer.clone(), status.clone());
    let metrics = MetricsCollector::new(pm.clone(), status.clone(), Duration::from_secs(5));
    let host = helm_proc::HostMonitor::new(Duration::from_secs(2));
    let scheduler = Scheduler::new(pm.clone()).await.unwrap();

    let (manual_tx, manual_rx) = tokio::sync::oneshot::channel::<()>();
    let (restart_tx, mut restart_rx) = tokio::sync::watch::channel(false);

    let mut state = make_state(db, pm, log_buffer, status, metrics, host, scheduler);
    state.dashboard_pin = pin.to_string();
    if restartable {
        state.restart_tx = Some(restart_tx);
    }

    let router = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = async {
                        if restartable {
                            loop {
                                if *restart_rx.borrow() {
                                    break;
                                }
                                if restart_rx.changed().await.is_err() {
                                    break;
                                }
                            }
                        } else {
                            std::future::pending::<()>().await;
                        }
                    } => {}
                    _ = async { let _ = manual_rx.await; } => {}
                }
            })
            .await
            .unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    TestServer {
        addr,
        shutdown_tx: Some(manual_tx),
        _tmp: tmp,
    }
}

#[tokio::test]
async fn pin_required_without_header_returns_401() {
    let srv = start_server("1234", false).await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/system/info", srv.base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn pin_accepts_correct_header_and_rejects_wrong_one() {
    let srv = start_server("1234", false).await;
    let client = reqwest::Client::new();

    let ok = client
        .get(format!("{}/api/system/info", srv.base_url()))
        .header("X-PIN", "1234")
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);

    let bad = client
        .get(format!("{}/api/system/info", srv.base_url()))
        .header("X-PIN", "9999")
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 401);
}

#[tokio::test]
async fn empty_pin_disables_auth_checks() {
    let srv = start_server("", false).await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/system/info", srv.base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn login_sets_cookie_and_cookie_is_accepted() {
    let srv = start_server("1234", false).await;
    let client = reqwest::Client::new();

    let login = client
        .post(format!("{}/api/login", srv.base_url()))
        .json(&serde_json::json!({"pin":"1234"}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), 200);
    let set_cookie = login
        .headers()
        .get(SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(set_cookie.starts_with("helm_pin=1234;"));
    let cookie_pair = set_cookie.split(';').next().unwrap_or_default().to_string();

    let resp = client
        .get(format!("{}/api/system/info", srv.base_url()))
        .header("Cookie", cookie_pair)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn restart_endpoint_triggers_graceful_shutdown() {
    let srv = start_server("", true).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/restart-server", srv.base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let health_url = format!("{}/api/health", srv.base_url());
    for _ in 0..30 {
        let health = client.get(&health_url).send().await;
        if health.is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("server did not shut down on restart trigger");
}
