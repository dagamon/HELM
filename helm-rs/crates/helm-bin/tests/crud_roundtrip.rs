//! CRUD round-trip: create → get → update → delete for services and scripts.

use serde_json::{json, Value};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};
use tempfile::TempDir;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(8);

use std::sync::atomic::{AtomicU16, Ordering};
static NEXT_PORT: AtomicU16 = AtomicU16::new(17050);
fn port_for_test() -> u16 {
    NEXT_PORT.fetch_add(1, Ordering::Relaxed)
}

struct HelmGuard {
    child: Child,
    port: u16,
}

impl HelmGuard {
    fn base(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for HelmGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn spawn_helm(db_path: &std::path::Path, port: u16) -> HelmGuard {
    let bin = env!("CARGO_BIN_EXE_helm");
    let child = Command::new(bin)
        .env("PORT", port.to_string())
        .env("DB_PATH", db_path.to_string_lossy().to_string())
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn helm");
    let guard = HelmGuard { child, port };

    let url = format!("http://127.0.0.1:{}/api/health", port);
    let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;
    let client = reqwest::Client::new();
    loop {
        if let Ok(r) = client.get(&url).send().await {
            if r.status().is_success() {
                return guard;
            }
        }
        if std::time::Instant::now() > deadline {
            panic!("helm did not become ready");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_crud_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let db_path: PathBuf = tmp.path().join("svc_crud.db");
    let helm = spawn_helm(&db_path, port_for_test()).await;
    let base = helm.base();
    let client = reqwest::Client::new();

    // CREATE
    let payload = json!({
        "name": "test_svc",
        "type": "shell",
        "command": "echo hi",
        "description": "round-trip"
    });
    let r = client
        .post(format!("{}/api/services", base))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let created: Value = r.json().await.unwrap();
    let id = created["id"].as_i64().unwrap();
    assert_eq!(created["name"], "test_svc");
    assert_eq!(created["status"], "stopped");
    assert!(created["pid"].is_null());

    // GET
    let r = client
        .get(format!("{}/api/services/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let got: Value = r.json().await.unwrap();
    assert_eq!(got["name"], "test_svc");
    assert_eq!(got["description"], "round-trip");

    // UPDATE
    let r = client
        .put(format!("{}/api/services/{}", base, id))
        .json(&json!({"description": "updated"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let updated: Value = r.json().await.unwrap();
    assert_eq!(updated["description"], "updated");

    // DELETE
    let r = client
        .delete(format!("{}/api/services/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);

    // GET after delete → 404
    let r = client
        .get(format!("{}/api/services/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);

    // 409 on duplicate stop: stop nonexistent service ID returns 500 placeholder (S6 will refine).
    // We only verify 404 here, which is the contract behaviour for missing resources.
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn script_crud_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("scr_crud.db");
    let helm = spawn_helm(&db_path, port_for_test()).await;
    let base = helm.base();
    let client = reqwest::Client::new();

    // CREATE
    let r = client
        .post(format!("{}/api/scripts", base))
        .json(&json!({
            "name": "test_scr",
            "command": "echo hello",
            "run_mode": "exec"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let created: Value = r.json().await.unwrap();
    let id = created["id"].as_i64().unwrap();
    assert_eq!(created["name"], "test_scr");
    assert_eq!(created["run_mode"], "exec");

    // GET
    let r = client
        .get(format!("{}/api/scripts/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // UPDATE
    let r = client
        .put(format!("{}/api/scripts/{}", base, id))
        .json(&json!({"description": "tweaked"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let updated: Value = r.json().await.unwrap();
    assert_eq!(updated["description"], "tweaked");

    // DELETE
    let r = client
        .delete(format!("{}/api/scripts/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);

    // 404 after delete
    let r = client
        .get(format!("{}/api/scripts/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_fields_round_trip() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("json.db");
    let helm = spawn_helm(&db_path, port_for_test()).await;
    let base = helm.base();
    let client = reqwest::Client::new();

    // Create service with args/env/tags/depends_on (all JSON-encoded fields)
    let r = client
        .post(format!("{}/api/services", base))
        .json(&json!({
            "name": "json_test",
            "type": "shell",
            "command": "echo",
            "args": ["a", "b", "c"],
            "env": {"FOO": "1", "BAR": "2"},
            "tags": ["x", "y"],
            "depends_on": [1, 2, 3]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let created: Value = r.json().await.unwrap();
    let id = created["id"].as_i64().unwrap();

    let r = client
        .get(format!("{}/api/services/{}", base, id))
        .send()
        .await
        .unwrap();
    let got: Value = r.json().await.unwrap();
    assert_eq!(got["args"], json!(["a", "b", "c"]));
    assert_eq!(got["env"]["FOO"], "1");
    assert_eq!(got["tags"], json!(["x", "y"]));
    assert_eq!(got["depends_on"], json!([1, 2, 3]));
}
