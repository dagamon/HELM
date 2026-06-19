//! Integration tests for HealthMonitor, LogRetention, MetricsCollector, Scheduler.

use helm_db::{repo, Db};
use helm_platform::RunMode;
use helm_proc::{
    HealthMonitor, LogBuffer, LogRetention, MetricsCollector, ProcessManager, RetentionConfig,
    Scheduler, SpawnSpec, StatusBroadcaster, StatusEvent,
};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

async fn make_db() -> (TempDir, Db) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("p.db");
    let db = Db::connect(path.to_str().unwrap()).await.unwrap();
    (tmp, db)
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
        name: "periodic-test".into(),
        depends_on: vec![],
    }
}

#[cfg(windows)]
fn long_sleep() -> &'static str {
    "ping -n 60 127.0.0.1 >NUL"
}
#[cfg(unix)]
fn long_sleep() -> &'static str {
    "sleep 60"
}

// ---------------------------------------------------------------------------
// LogRetention
// ---------------------------------------------------------------------------

#[tokio::test]
async fn retention_caps_total_rows() {
    let (_tmp, db) = make_db().await;
    for i in 0..10 {
        repo::output_logs::insert(&db.pool, "service", 1, "stdout", &format!("line-{i}"))
            .await
            .unwrap();
    }
    let mut r = LogRetention::new(
        db.clone(),
        RetentionConfig {
            retention_days: 0,
            max_rows_total: 3,
            max_rows_per_entity: 0,
            cleanup_interval: Duration::from_secs(60),
            vacuum_interval: Duration::ZERO,
        },
    );
    r.run_once().await.unwrap();
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM output_logs")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 3);
}

#[tokio::test]
async fn retention_caps_per_entity() {
    let (_tmp, db) = make_db().await;
    for i in 0..5 {
        repo::output_logs::insert(&db.pool, "service", 1, "stdout", &format!("a-{i}"))
            .await
            .unwrap();
        repo::output_logs::insert(&db.pool, "service", 2, "stdout", &format!("b-{i}"))
            .await
            .unwrap();
    }
    let mut r = LogRetention::new(
        db.clone(),
        RetentionConfig {
            retention_days: 0,
            max_rows_total: 0,
            max_rows_per_entity: 2,
            cleanup_interval: Duration::from_secs(60),
            vacuum_interval: Duration::ZERO,
        },
    );
    r.run_once().await.unwrap();
    let a = repo::output_logs::recent(&db.pool, "service", 1, 100)
        .await
        .unwrap();
    let b = repo::output_logs::recent(&db.pool, "service", 2, 100)
        .await
        .unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 2);
}

// ---------------------------------------------------------------------------
// HealthMonitor
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_monitor_broadcasts_healthy_and_unhealthy() {
    use httpmock::prelude::*;
    let (_tmp, db) = make_db().await;
    let server = MockServer::start_async().await;
    let _ok = server
        .mock_async(|w, t| {
            w.method(GET).path("/ok");
            t.status(200);
        })
        .await;
    let _fail = server
        .mock_async(|w, t| {
            w.method(GET).path("/bad");
            t.status(503);
        })
        .await;

    // Two services with health_check_url
    use helm_core::models::ServiceCreate;
    let svc_ok = ServiceCreate {
        name: "ok".into(),
        description: None,
        r#type: "url".into(),
        command: None,
        cwd: None,
        venv_path: None,
        args: None,
        env: None,
        url: None,
        health_check_url: Some(server.url("/ok")),
        health_check_interval: 30,
        auto_start: false,
        restart_on_crash: false,
        platform: "all".into(),
        tags: None,
        depends_on: None,
        webhook_url: None,
        manifest_path: None,
        binary_path: None,
        cargo_profile: None,
        cargo_features: None,
        prebuild: false,
    };
    let svc_bad = ServiceCreate {
        name: "bad".into(),
        description: None,
        r#type: "url".into(),
        command: None,
        cwd: None,
        venv_path: None,
        args: None,
        env: None,
        url: None,
        health_check_url: Some(server.url("/bad")),
        health_check_interval: 30,
        auto_start: false,
        restart_on_crash: false,
        platform: "all".into(),
        tags: None,
        depends_on: None,
        webhook_url: None,
        manifest_path: None,
        binary_path: None,
        cargo_profile: None,
        cargo_features: None,
        prebuild: false,
    };
    let id_ok = repo::services::create(&db.pool, &svc_ok).await.unwrap();
    let id_bad = repo::services::create(&db.pool, &svc_bad).await.unwrap();

    let status = StatusBroadcaster::new(32);
    let mut rx = status.subscribe();
    let monitor = HealthMonitor::new(db.clone(), status.clone(), 4, Duration::from_secs(60));
    // Run one cycle directly (via spawn loop with a quick first iteration).
    let handle = monitor.spawn();

    // Collect events for up to 5s
    let mut seen_ok = false;
    let mut seen_bad = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline && !(seen_ok && seen_bad) {
        if let Ok(Ok(ev)) = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
            if ev.entity_id == id_ok && ev.status == "healthy" {
                seen_ok = true;
            }
            if ev.entity_id == id_bad && ev.status == "unhealthy" {
                seen_bad = true;
            }
        }
    }
    handle.abort();
    assert!(seen_ok, "missed healthy event");
    assert!(seen_bad, "missed unhealthy event");
}

// ---------------------------------------------------------------------------
// MetricsCollector
// ---------------------------------------------------------------------------

#[tokio::test]
async fn metrics_collector_records_snapshots_for_running_process() {
    let (_tmp, db) = make_db().await;
    let lb = LogBuffer::new(100);
    let status = StatusBroadcaster::new(32);
    let pm = ProcessManager::new(db.clone(), lb, status.clone());
    let metrics = MetricsCollector::new(pm.clone(), status.clone(), Duration::from_millis(200));
    metrics.clone().spawn();

    let mut spec = shell_spec("service", 100, long_sleep());
    spec.entity_type = "service".into();
    let mp = pm.spawn(spec).await.unwrap();
    sleep(Duration::from_millis(1500)).await; // two collect cycles
    let snaps = metrics.snapshots("service_100");
    assert!(!snaps.is_empty(), "no snapshots after 1.5s");
    pm.terminate("service_100").await.unwrap();
    let _ = mp;
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

#[tokio::test]
async fn scheduler_validates_cron_expressions() {
    assert!(Scheduler::validate_cron("* * * * *"));
    assert!(Scheduler::validate_cron("0 0 * * *"));
    assert!(!Scheduler::validate_cron("not a cron"));
}

#[tokio::test]
async fn scheduler_triggers_script_at_next_minute() {
    let (_tmp, db) = make_db().await;
    let lb = LogBuffer::new(100);
    let status = StatusBroadcaster::new(32);
    let pm = ProcessManager::new(db.clone(), lb, status);

    // Create a script that we'd cron-schedule.
    use helm_core::models::ScriptCreate;
    #[cfg(windows)]
    let script_cmd = "echo cron-fired".to_string();
    #[cfg(unix)]
    let script_cmd = "echo cron-fired".to_string();

    let script = ScriptCreate {
        name: "cron".into(),
        description: None,
        command: script_cmd,
        run_mode: "shell".into(),
        cwd: None,
        args: None,
        platform: "all".into(),
        tags: None,
        cron_schedule: Some("* * * * * *".into()), // every second (6-field passthrough)
        cron_enabled: true,
    };
    let script_id = repo::scripts::create(&db.pool, &script).await.unwrap();

    let scheduler = Scheduler::new(pm.clone()).await.unwrap();
    let added = scheduler.load_all(&db).await.unwrap();
    assert!(added >= 1);

    // Wait up to ~3s for at least one run_logs row to appear for this script.
    let mut got = false;
    for _ in 0..60 {
        sleep(Duration::from_millis(100)).await;
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM run_logs WHERE entity_type='script' AND entity_id=? LIMIT 1",
        )
        .bind(script_id)
        .fetch_optional(&db.pool)
        .await
        .unwrap();
        if row.is_some() {
            got = true;
            break;
        }
    }
    assert!(got, "scheduler did not fire script within 6s");

    let next = scheduler.next_run(script_id).await;
    assert!(next.is_some(), "expected next_run to be Some");

    scheduler.remove(script_id).await.unwrap();
    drop(_tmp);
    let _ = StatusEvent::new("script", script_id, "stopped", None);
}
