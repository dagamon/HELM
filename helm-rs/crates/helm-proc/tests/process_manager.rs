//! Integration tests for ProcessManager. Uses shell built-ins (cmd / sh) for
//! cross-platform child commands so we don't depend on a real `python` binary
//! (Windows ships a WindowsApps store stub that interferes).

use helm_db::{repo, Db};
use helm_platform::RunMode;
use helm_proc::{LogBuffer, ProcessManager, SpawnSpec, StatusBroadcaster};
use std::{sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::time::sleep;

struct TestRig {
    _tmp: TempDir,
    db: Db,
    pm: Arc<ProcessManager>,
    status: Arc<StatusBroadcaster>,
}

async fn rig() -> TestRig {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let db = Db::connect(db_path.to_str().unwrap()).await.unwrap();
    let lb = LogBuffer::new(500);
    let status = StatusBroadcaster::new(64);
    let pm = ProcessManager::new(db.clone(), lb, status.clone());
    TestRig {
        _tmp: tmp,
        db,
        pm,
        status,
    }
}

fn shell(script: &str) -> SpawnSpec {
    SpawnSpec {
        entity_type: "script".into(),
        entity_id: 0,
        command: script.into(),
        args: vec![],
        cwd: None,
        env: None,
        venv_path: None,
        run_mode: RunMode::Shell,
        restart_on_crash: false,
        webhook_url: None,
        name: "test".into(),
        depends_on: vec![],
    }
}

#[cfg(windows)]
fn two_echoes() -> &'static str {
    "echo line-a&echo line-b"
}
#[cfg(unix)]
fn two_echoes() -> &'static str {
    "echo line-a; echo line-b"
}

#[cfg(windows)]
fn one_echo(text: &str) -> String {
    format!("echo {}", text)
}
#[cfg(unix)]
fn one_echo(text: &str) -> String {
    format!("echo {}", text)
}

#[cfg(windows)]
fn exit_code(code: i32) -> String {
    format!("exit /b {}", code)
}
#[cfg(unix)]
fn exit_code(code: i32) -> String {
    format!("exit {}", code)
}

#[cfg(windows)]
fn long_sleep() -> &'static str {
    "ping -n 60 127.0.0.1 >NUL"
}
#[cfg(unix)]
fn long_sleep() -> &'static str {
    "sleep 60"
}

async fn wait_for_status(db: &Db, run_log_id: i64, target: &str, max_ms: u64) -> Option<String> {
    let step = 50u64;
    let mut waited = 0u64;
    loop {
        if let Some(row) = repo::run_logs::get(&db.pool, run_log_id).await.unwrap() {
            if let Some(s) = &row.status {
                if s == target {
                    return Some(s.clone());
                }
            }
        }
        if waited >= max_ms {
            return None;
        }
        sleep(Duration::from_millis(step)).await;
        waited += step;
    }
}

#[tokio::test]
async fn spawn_records_success() {
    let r = rig().await;
    let mut spec = shell(&one_echo("hello"));
    spec.entity_id = 1;
    let mp = r.pm.spawn(spec).await.unwrap();
    let status = wait_for_status(&r.db, mp.run_log_id, "success", 8000).await;
    assert_eq!(status.as_deref(), Some("success"));
}

#[tokio::test]
async fn spawn_captures_stdout_into_logs_table() {
    let r = rig().await;
    let mut spec = shell(two_echoes());
    spec.entity_id = 2;
    let mp = r.pm.spawn(spec).await.unwrap();
    let _ = wait_for_status(&r.db, mp.run_log_id, "success", 8000).await;
    sleep(Duration::from_millis(200)).await;
    let lines = repo::output_logs::recent(&r.db.pool, "script", 2, 100)
        .await
        .unwrap();
    let texts: Vec<String> = lines
        .iter()
        .map(|l| l.line.trim_end().to_string())
        .collect();
    assert!(texts.iter().any(|t| t == "line-a"), "got: {:?}", texts);
    assert!(texts.iter().any(|t| t == "line-b"), "got: {:?}", texts);
}

#[tokio::test]
async fn crash_records_crashed_status() {
    let r = rig().await;
    let mut spec = shell(&exit_code(2));
    spec.entity_id = 3;
    let mp = r.pm.spawn(spec).await.unwrap();
    let status = wait_for_status(&r.db, mp.run_log_id, "crashed", 8000).await;
    assert_eq!(status.as_deref(), Some("crashed"));
    let row = repo::run_logs::get(&r.db.pool, mp.run_log_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.exit_code, Some(2));
}

#[tokio::test]
async fn terminate_marks_stopped() {
    let r = rig().await;
    let mut spec = shell(long_sleep());
    spec.entity_type = "service".into();
    spec.entity_id = 4;
    let mp = r.pm.spawn(spec).await.unwrap();
    sleep(Duration::from_millis(500)).await;
    let key = format!("service_{}", mp.entity_id);
    assert_eq!(r.pm.status(&key), "running");
    r.pm.terminate(&key).await.unwrap();
    let status = wait_for_status(&r.db, mp.run_log_id, "stopped", 10_000).await;
    assert_eq!(status.as_deref(), Some("stopped"));
    assert_eq!(r.pm.status(&key), "stopped");
}

#[tokio::test]
async fn depends_on_blocks_when_dep_not_running() {
    let r = rig().await;
    let mut spec = shell(&one_echo("child"));
    spec.entity_type = "service".into();
    spec.entity_id = 99;
    spec.depends_on = vec![999];
    let err = r.pm.spawn(spec).await.unwrap_err();
    assert!(err.to_string().contains("Dependency"), "got: {err}");
}

#[tokio::test]
async fn depends_on_passes_when_dep_running() {
    let r = rig().await;
    let mut dep = shell(long_sleep());
    dep.entity_type = "service".into();
    dep.entity_id = 10;
    let dep_mp = r.pm.spawn(dep).await.unwrap();
    sleep(Duration::from_millis(500)).await;
    assert_eq!(r.pm.status("service_10"), "running");

    let mut spec = shell(&one_echo("ok"));
    spec.entity_type = "service".into();
    spec.entity_id = 11;
    spec.depends_on = vec![10];
    let mp = r.pm.spawn(spec).await.unwrap();
    let s = wait_for_status(&r.db, mp.run_log_id, "success", 8000).await;
    assert_eq!(s.as_deref(), Some("success"));

    r.pm.terminate("service_10").await.unwrap();
    let _ = wait_for_status(&r.db, dep_mp.run_log_id, "stopped", 10_000).await;
}

#[tokio::test]
async fn webhook_fires_on_crash() {
    use httpmock::prelude::*;
    let server = MockServer::start_async().await;
    let m = server
        .mock_async(|when, then| {
            when.method(POST).path("/hook");
            then.status(200);
        })
        .await;

    let r = rig().await;
    let mut spec = shell(&exit_code(7));
    spec.entity_type = "service".into();
    spec.entity_id = 20;
    spec.webhook_url = Some(server.url("/hook"));
    let mp = r.pm.spawn(spec).await.unwrap();
    let _ = wait_for_status(&r.db, mp.run_log_id, "crashed", 8000).await;
    for _ in 0..60 {
        if m.hits_async().await > 0 {
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }
    m.assert_hits_async(1).await;
}

#[tokio::test]
async fn intentional_stop_does_not_broadcast_crashed() {
    use tokio::sync::broadcast::error::TryRecvError;
    let r = rig().await;
    let mut rx = r.status.subscribe();

    let mut spec = shell(long_sleep());
    spec.entity_type = "service".into();
    spec.entity_id = 30;
    let mp = r.pm.spawn(spec).await.unwrap();
    sleep(Duration::from_millis(500)).await;
    r.pm.terminate("service_30").await.unwrap();
    let _ = wait_for_status(&r.db, mp.run_log_id, "stopped", 10_000).await;
    sleep(Duration::from_millis(300)).await;

    let mut saw_stopped = false;
    loop {
        match rx.try_recv() {
            Ok(ev) => {
                assert_ne!(ev.status, "crashed", "intentional stop emitted crashed");
                if ev.status == "stopped" {
                    saw_stopped = true;
                }
            }
            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => break,
            Err(TryRecvError::Lagged(_)) => continue,
        }
    }
    assert!(saw_stopped, "expected stopped event");
}

#[tokio::test]
async fn terminate_kills_descendant_process_tree() {
    use sysinfo::{Pid, System};

    let r = rig().await;
    // Parent shell spawns one or more long-running children. ping is a reliable
    // OS-native long-runner on both Windows and Linux.
    let script = if cfg!(windows) {
        // start /b detaches into background; the parent keeps waiting via ping too
        "start /b ping -n 60 127.0.0.1 >NUL & ping -n 60 127.0.0.1 >NUL"
    } else {
        "ping -c 60 127.0.0.1 > /dev/null & ping -c 60 127.0.0.1 > /dev/null"
    };
    let mut spec = shell(script);
    spec.entity_type = "service".into();
    spec.entity_id = 50;
    let mp = r.pm.spawn(spec).await.unwrap();
    sleep(Duration::from_millis(800)).await;

    let parent_pid = mp.pid;

    fn collect_descendants(sys: &System, root: u32) -> Vec<u32> {
        let mut out = vec![];
        let mut frontier = vec![Pid::from_u32(root)];
        while let Some(p) = frontier.pop() {
            for (pid, proc) in sys.processes() {
                if proc.parent() == Some(p) {
                    out.push(pid.as_u32());
                    frontier.push(*pid);
                }
            }
        }
        out
    }

    let mut sys = System::new_all();
    sys.refresh_all();
    let before = collect_descendants(&sys, parent_pid);
    assert!(
        !before.is_empty(),
        "expected at least one descendant before kill"
    );

    r.pm.terminate("service_50").await.unwrap();
    let _ = wait_for_status(&r.db, mp.run_log_id, "stopped", 10_000).await;
    // Allow a beat for OS cleanup.
    sleep(Duration::from_millis(800)).await;

    sys.refresh_all();
    let still_alive: Vec<u32> = before
        .iter()
        .copied()
        .filter(|pid| sys.process(Pid::from_u32(*pid)).is_some())
        .collect();
    assert!(
        still_alive.is_empty(),
        "descendants leaked after terminate: {:?}",
        still_alive
    );
}

#[tokio::test]
async fn restart_on_crash_respawns_service() {
    let r = rig().await;
    let mut spec = shell(&exit_code(1));
    spec.entity_type = "service".into();
    spec.entity_id = 40;
    spec.restart_on_crash = true;
    let mp = r.pm.spawn(spec).await.unwrap();
    let _ = wait_for_status(&r.db, mp.run_log_id, "crashed", 8000).await;

    let mut second_id: Option<i64> = None;
    for _ in 0..100 {
        sleep(Duration::from_millis(100)).await;
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM run_logs WHERE entity_type='service' AND entity_id=40 AND id > ? ORDER BY id ASC LIMIT 1",
        )
        .bind(mp.run_log_id)
        .fetch_optional(&r.db.pool)
        .await
        .unwrap();
        if let Some((id,)) = row {
            second_id = Some(id);
            break;
        }
    }
    let id = second_id.expect("restart did not happen within 10s");
    let _ = wait_for_status(&r.db, id, "crashed", 8000).await;
    let _ = r.pm.terminate("service_40").await;
}
