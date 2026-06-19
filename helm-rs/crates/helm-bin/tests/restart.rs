use std::{
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(8);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(8);
static NEXT_PORT: AtomicU16 = AtomicU16::new(17120);

fn next_port() -> u16 {
    NEXT_PORT.fetch_add(1, Ordering::Relaxed)
}

struct HelmProc {
    child: Child,
    port: u16,
}

impl HelmProc {
    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for HelmProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn spawn_helm(db_path: &std::path::Path, port: u16) -> HelmProc {
    let bin = env!("CARGO_BIN_EXE_helm");
    let child = Command::new(bin)
        .env("PORT", port.to_string())
        .env("DB_PATH", db_path.to_string_lossy().to_string())
        .env("DASHBOARD_PIN", "")
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn helm");
    let proc = HelmProc { child, port };

    let client = reqwest::Client::new();
    let health = format!("{}/api/health", proc.base_url());
    let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;
    loop {
        if let Ok(resp) = client.get(&health).send().await {
            if resp.status().is_success() {
                return proc;
            }
        }
        if std::time::Instant::now() > deadline {
            panic!("helm did not become ready");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_server_exits_process_with_zero_code() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("restart.db");
    let port = next_port();
    let mut helm = spawn_helm(&db_path, port).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/restart-server", helm.base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let deadline = std::time::Instant::now() + SHUTDOWN_TIMEOUT;
    loop {
        match helm.child.try_wait() {
            Ok(Some(status)) => {
                assert_eq!(status.code(), Some(0));
                return;
            }
            Ok(None) => {
                if std::time::Instant::now() > deadline {
                    panic!("helm did not exit after restart request");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => panic!("failed waiting for helm: {e}"),
        }
    }
}
