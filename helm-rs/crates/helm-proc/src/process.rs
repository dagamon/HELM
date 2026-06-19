//! Process lifecycle manager. Port of `server/services/process_manager.py`.
//!
//! Spawns child processes via `helm-platform::build_command`, pipes stdout/stderr
//! into the [`LogBuffer`] + `output_logs` table + per-key broadcast channel, and
//! records lifecycle events in `run_logs` + the global [`StatusBroadcaster`].
//!
//! Termination uses the OS-native process-tree mechanism: JobObject on Windows,
//! `killpg` on Unix. See [`crate::job`].

use crate::{
    job,
    log_buffer::{LogBuffer, LogEntry},
    status::{StatusBroadcaster, StatusEvent},
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use dashmap::DashMap;
use helm_db::{repo, Db};
use helm_platform::{build_command, build_venv_env, RunMode};
use std::future::Future;
use std::pin::Pin;
use std::{
    collections::HashMap,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
#[cfg(unix)]
use tokio::time::timeout;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStderr, ChildStdout, Command},
    sync::{broadcast, Mutex},
};

type SpawnFuture = Pin<Box<dyn Future<Output = Result<ManagedProcess>> + Send>>;
use tracing::{info, warn};

/// Per-line broadcast envelope used by `/ws/logs/{type}/{id}` subscribers.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogMsg {
    pub stream: String,
    pub text: String,
    pub ts: String,
}

#[derive(Debug, Clone)]
pub struct ManagedProcess {
    pub key: String,
    pub entity_type: String,
    pub entity_id: i64,
    pub pid: u32,
    pub started_at: String,
    pub run_log_id: i64,
    pub name: String,
}

/// Input bundle for `spawn`. Fewer positional args than the Python equivalent.
pub struct SpawnSpec {
    pub entity_type: String,
    pub entity_id: i64,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub venv_path: Option<String>,
    pub run_mode: RunMode,
    pub restart_on_crash: bool,
    pub webhook_url: Option<String>,
    pub name: String,
    pub depends_on: Vec<i64>,
}

impl SpawnSpec {
    pub fn key(&self) -> String {
        format!("{}_{}", self.entity_type, self.entity_id)
    }
}

struct Slot {
    info: ManagedProcess,
    intentional_stop: Arc<AtomicBool>,
    #[cfg(windows)]
    job: Mutex<Option<job::JobHandle>>,
}

pub struct ProcessManager {
    db: Db,
    log_buffer: Arc<LogBuffer>,
    status: Arc<StatusBroadcaster>,
    procs: DashMap<String, Arc<Slot>>,
    log_channels: DashMap<String, broadcast::Sender<LogMsg>>,
    http: reqwest::Client,
}

impl ProcessManager {
    pub fn new(db: Db, log_buffer: Arc<LogBuffer>, status: Arc<StatusBroadcaster>) -> Arc<Self> {
        Arc::new(Self {
            db,
            log_buffer,
            status,
            procs: DashMap::new(),
            log_channels: DashMap::new(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client"),
        })
    }

    // ---- public API ----

    pub fn status(&self, key: &str) -> &'static str {
        if self.procs.contains_key(key) {
            "running"
        } else {
            "stopped"
        }
    }

    pub fn get(&self, key: &str) -> Option<ManagedProcess> {
        self.procs.get(key).map(|s| s.info.clone())
    }

    /// Snapshot of every currently-running managed process. Cheap clone of
    /// `ManagedProcess` (a few fields).
    pub fn running_processes(&self) -> Vec<ManagedProcess> {
        self.procs.iter().map(|e| e.value().info.clone()).collect()
    }

    /// Subscribe to per-entity log stream. Returns a receiver that yields every
    /// new line; pair with [`LogBuffer::recent`] for history on connect.
    pub fn subscribe_logs(&self, key: &str) -> broadcast::Receiver<LogMsg> {
        let entry = self
            .log_channels
            .entry(key.to_string())
            .or_insert_with(|| broadcast::channel(256).0);
        entry.subscribe()
    }

    /// Spawn a managed process. Validates `depends_on`, writes the `run_logs` row,
    /// fires `running` status, and starts the stdout/stderr/wait tasks.
    ///
    /// Returns a boxed Send future to break the type-inference cycle between
    /// `spawn` → `wait_for_exit` → `spawn` (restart-on-crash path).
    pub fn spawn(self: &Arc<Self>, spec: SpawnSpec) -> SpawnFuture {
        let me = self.clone();
        Box::pin(async move { me.spawn_inner(spec).await })
    }

    async fn spawn_inner(self: Arc<Self>, spec: SpawnSpec) -> Result<ManagedProcess> {
        let key = spec.key();
        if self.procs.contains_key(&key) {
            return Err(anyhow!("{key} is already running"));
        }
        for dep in &spec.depends_on {
            let dep_key = format!("service_{dep}");
            if self.status(&dep_key) != "running" {
                return Err(anyhow!(
                    "Dependency service {dep} is not running (status: {})",
                    self.status(&dep_key)
                ));
            }
        }

        let argv = build_command(
            &spec.command,
            &spec.args,
            None,
            spec.venv_path.as_deref(),
            spec.run_mode,
        )?;
        if argv.is_empty() {
            return Err(anyhow!("empty argv"));
        }

        let env = build_venv_env(spec.venv_path.as_deref(), spec.env.as_ref());

        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(c) = &spec.cwd {
            cmd.current_dir(c);
        }
        if let Some(e) = env {
            cmd.env_clear().envs(e);
        }

        #[cfg(windows)]
        {
            // CREATE_NEW_PROCESS_GROUP — isolate Ctrl+C/Break from HELM.
            // tokio::process::Command exposes creation_flags as an inherent method on Windows.
            cmd.creation_flags(0x00000200);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| job::set_session_leader());
            }
        }

        let mut child = cmd.spawn().map_err(|e| anyhow!("spawn failed: {e}"))?;
        let pid = child
            .id()
            .ok_or_else(|| anyhow!("child has no pid (already exited?)"))?;

        #[cfg(windows)]
        let job_handle = match job::JobHandle::create_and_assign(pid) {
            Ok(h) => Some(h),
            Err(e) => {
                warn!("JobObject assign failed for pid {pid}: {e}; will rely on taskkill");
                None
            }
        };

        let run_log_id =
            repo::run_logs::insert(&self.db.pool, &spec.entity_type, spec.entity_id, pid).await?;

        let started_at = Utc::now().to_rfc3339();
        let info = ManagedProcess {
            key: key.clone(),
            entity_type: spec.entity_type.clone(),
            entity_id: spec.entity_id,
            pid,
            started_at: started_at.clone(),
            run_log_id,
            name: spec.name.clone(),
        };
        let slot = Arc::new(Slot {
            info: info.clone(),
            intentional_stop: Arc::new(AtomicBool::new(false)),
            #[cfg(windows)]
            job: Mutex::new(job_handle),
        });
        self.procs.insert(key.clone(), slot.clone());

        // log channel
        let log_tx = self
            .log_channels
            .entry(key.clone())
            .or_insert_with(|| broadcast::channel(256).0)
            .clone();

        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");

        let pm = self.clone();
        let k = key.clone();
        let etype = spec.entity_type.clone();
        let eid = spec.entity_id;
        let tx = log_tx.clone();
        tokio::spawn(async move {
            pm.read_stdout(stdout, k, etype, eid, "stdout", tx).await;
        });
        let pm = self.clone();
        let k = key.clone();
        let etype = spec.entity_type.clone();
        let eid = spec.entity_id;
        let tx = log_tx.clone();
        tokio::spawn(async move {
            pm.read_stderr(stderr, k, etype, eid, "stderr", tx).await;
        });

        // wait_for_exit task takes ownership of child
        let pm = self.clone();
        let slot_w = slot.clone();
        let spec_for_restart = RespawnPlan::from_spec(&spec);
        tokio::spawn(async move {
            pm.wait_for_exit(child, slot_w, spec_for_restart).await;
        });

        self.status.send(StatusEvent::new(
            &spec.entity_type,
            spec.entity_id,
            "running",
            Some(pid),
        ));
        info!(key = %key, pid = pid, "process started");
        Ok(info)
    }

    /// Intentional termination. Marks the slot, kills the tree, waits for the
    /// child to exit, broadcasts `stopped`.
    pub async fn terminate(&self, key: &str) -> Result<()> {
        let slot = self
            .procs
            .remove(key)
            .map(|(_, s)| s)
            .ok_or_else(|| anyhow!("{key} is not running"))?;
        slot.intentional_stop.store(true, Ordering::SeqCst);

        let pid = slot.info.pid;

        #[cfg(windows)]
        {
            let mut held = slot.job.lock().await;
            let job_ok = if let Some(h) = held.as_ref() {
                h.terminate().is_ok()
            } else {
                false
            };
            *held = None; // drop handle either way
            if !job_ok {
                let killed = job::taskkill_tree(pid).await;
                if !killed {
                    warn!("taskkill failed for pid {pid}");
                }
            }
        }
        #[cfg(unix)]
        {
            use nix::sys::signal::Signal;
            if let Err(e) = job::kill_process_group(pid, Signal::SIGTERM) {
                warn!("SIGTERM killpg({pid}) failed: {e}");
            }
            // wait up to 5s then SIGKILL — done inside wait_for_exit via timeout.
        }

        self.status.send(StatusEvent::new(
            &slot.info.entity_type,
            slot.info.entity_id,
            "stopped",
            None,
        ));
        Ok(())
    }

    pub async fn shutdown(&self) {
        let keys: Vec<String> = self.procs.iter().map(|e| e.key().clone()).collect();
        for k in keys {
            if let Err(e) = self.terminate(&k).await {
                warn!("shutdown: terminate({k}) failed: {e}");
            }
        }
    }

    // ---- internal ----

    async fn read_stdout(
        self: Arc<Self>,
        stream: ChildStdout,
        key: String,
        entity_type: String,
        entity_id: i64,
        stream_name: &str,
        tx: broadcast::Sender<LogMsg>,
    ) {
        let reader = BufReader::new(stream);
        self.pump_lines(reader.lines(), key, entity_type, entity_id, stream_name, tx)
            .await;
    }

    async fn read_stderr(
        self: Arc<Self>,
        stream: ChildStderr,
        key: String,
        entity_type: String,
        entity_id: i64,
        stream_name: &str,
        tx: broadcast::Sender<LogMsg>,
    ) {
        let reader = BufReader::new(stream);
        self.pump_lines(reader.lines(), key, entity_type, entity_id, stream_name, tx)
            .await;
    }

    async fn pump_lines<R>(
        self: Arc<Self>,
        mut lines: tokio::io::Lines<BufReader<R>>,
        key: String,
        entity_type: String,
        entity_id: i64,
        stream_name: &str,
        tx: broadcast::Sender<LogMsg>,
    ) where
        R: tokio::io::AsyncRead + Unpin,
    {
        while let Ok(Some(line)) = lines.next_line().await {
            let ts = Utc::now().to_rfc3339();
            let entry = LogEntry {
                stream: stream_name.to_string(),
                text: line.clone(),
                ts: ts.clone(),
            };
            self.log_buffer.append(&key, entry);

            if let Err(e) = repo::output_logs::insert(
                &self.db.pool,
                &entity_type,
                entity_id,
                stream_name,
                &line,
            )
            .await
            {
                warn!("output_logs insert failed for {key}: {e}");
            }

            let _ = tx.send(LogMsg {
                stream: stream_name.into(),
                text: line,
                ts,
            });
        }
    }

    async fn wait_for_exit(
        self: Arc<Self>,
        mut child: tokio::process::Child,
        slot: Arc<Slot>,
        respawn: RespawnPlan,
    ) {
        // On Unix, if terminate() already fired SIGTERM, give the child 5s to die
        // before escalating to SIGKILL. Windows kills synchronously via JobObject.
        let exit;
        #[cfg(unix)]
        {
            if slot.intentional_stop.load(Ordering::SeqCst) {
                exit = match timeout(Duration::from_secs(5), child.wait()).await {
                    Ok(r) => r,
                    Err(_) => {
                        warn!("child {} did not exit on SIGTERM; SIGKILL", slot.info.pid);
                        let _ = child.kill().await;
                        child.wait().await
                    }
                };
            } else {
                exit = child.wait().await;
            }
        }
        #[cfg(not(unix))]
        {
            exit = child.wait().await;
        }

        let exit_code = exit.ok().and_then(|s| s.code());
        let intentional = slot.intentional_stop.load(Ordering::SeqCst);
        let key = slot.info.key.clone();

        // If the manager already removed this slot (intentional stop), only update DB.
        let still_present = self
            .procs
            .get(&key)
            .map(|s| Arc::ptr_eq(&*s, &slot))
            .unwrap_or(false);

        if intentional || !still_present {
            if let Err(e) = repo::run_logs::update_stopped(
                &self.db.pool,
                slot.info.run_log_id,
                exit_code,
                "stopped",
            )
            .await
            {
                warn!("run_logs update_stopped failed: {e}");
            }
            info!(key=%key, code=?exit_code, "process intentionally stopped");
            return;
        }

        let status = if exit_code == Some(0) {
            "success"
        } else {
            "crashed"
        };
        if let Err(e) =
            repo::run_logs::update_stopped(&self.db.pool, slot.info.run_log_id, exit_code, status)
                .await
        {
            warn!("run_logs update_stopped failed: {e}");
        }
        self.procs.remove(&key);

        self.status.send(StatusEvent::new(
            &slot.info.entity_type,
            slot.info.entity_id,
            status,
            None,
        ));
        info!(key=%key, code=?exit_code, status=status, "process exited");

        if status == "crashed" {
            if let Some(url) = &respawn.webhook_url {
                if slot.info.entity_type == "service" {
                    let payload = serde_json::json!({
                        "name": respawn.name,
                        "status": "crashed",
                        "exit_code": exit_code,
                        "pid": slot.info.pid,
                        "timestamp": Utc::now().to_rfc3339(),
                    });
                    let http = self.http.clone();
                    let url = url.clone();
                    tokio::spawn(async move {
                        match http.post(&url).json(&payload).send().await {
                            Ok(r) => info!("webhook {url} → {}", r.status()),
                            Err(e) => warn!("webhook {url} failed: {e}"),
                        }
                    });
                }
            }

            if respawn.restart && slot.info.entity_type == "service" {
                info!("restarting {key} in 3s");
                tokio::time::sleep(Duration::from_secs(3)).await;
                let pm = self.clone();
                let spec = respawn.into_spec(slot.info.entity_type.clone(), slot.info.entity_id);
                tokio::spawn(async move {
                    if let Err(e) = pm.spawn(spec).await {
                        warn!("restart failed: {e}");
                    }
                });
            }
        }
    }
}

#[derive(Clone)]
struct RespawnPlan {
    restart: bool,
    command: String,
    args: Vec<String>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
    venv_path: Option<String>,
    run_mode: RunMode,
    webhook_url: Option<String>,
    name: String,
    depends_on: Vec<i64>,
}

impl RespawnPlan {
    fn from_spec(s: &SpawnSpec) -> Self {
        Self {
            restart: s.restart_on_crash,
            command: s.command.clone(),
            args: s.args.clone(),
            cwd: s.cwd.clone(),
            env: s.env.clone(),
            venv_path: s.venv_path.clone(),
            run_mode: s.run_mode,
            webhook_url: s.webhook_url.clone(),
            name: s.name.clone(),
            depends_on: s.depends_on.clone(),
        }
    }
    fn into_spec(self, entity_type: String, entity_id: i64) -> SpawnSpec {
        SpawnSpec {
            entity_type,
            entity_id,
            command: self.command,
            args: self.args,
            cwd: self.cwd,
            env: self.env,
            venv_path: self.venv_path,
            run_mode: self.run_mode,
            restart_on_crash: self.restart,
            webhook_url: self.webhook_url,
            name: self.name,
            depends_on: self.depends_on,
        }
    }
}
