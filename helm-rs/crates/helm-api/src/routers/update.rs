//! Git-based self-update.
//!
//! `GET  /api/update/check`  — `git fetch` + compare local HEAD with upstream.
//! `POST /api/update/apply`  — write an updater script and spawn it detached so
//! it survives the service restart, then `git pull` + rebuild backend/frontend
//! and restart the HELM service (NSSM on Windows, systemd on Linux).
//!
//! Repo dir resolves from `HELM_REPO_DIR`, else the workspace root (three
//! parents up from this crate's manifest). Service name from
//! `HELM_UPDATE_SERVICE_NAME` (default `HELM`).

use crate::{error::ApiError, state::AppState};
use axum::{routing::{get, post}, Json, Router};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/update/check", get(check))
        .route("/api/update/apply", post(apply))
}

fn repo_dir() -> PathBuf {
    if let Ok(p) = std::env::var("HELM_REPO_DIR") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or(manifest)
}

fn service_name() -> String {
    std::env::var("HELM_UPDATE_SERVICE_NAME").unwrap_or_else(|_| "HELM".into())
}

async fn git(dir: &Path, args: &[&str]) -> Result<String, ApiError> {
    // `-c safe.directory=*` avoids git's "dubious ownership" refusal when HELM
    // runs as a different user (e.g. SYSTEM under NSSM) than the repo's owner.
    let out = Command::new("git")
        .arg("-c")
        .arg("safe.directory=*")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .await
        .map_err(|e| ApiError::Internal(format!("git spawn failed: {e}")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(ApiError::Internal(format!(
            "git {} failed: {}",
            args.join(" "),
            err.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[derive(Serialize)]
pub struct UpdateStatus {
    pub branch: String,
    pub current: String,
    pub current_short: String,
    pub latest: String,
    pub latest_short: String,
    pub behind: u32,
    pub ahead: u32,
    pub update_available: bool,
    pub latest_subject: String,
    pub checked_at: i64,
}

async fn check() -> Result<Json<UpdateStatus>, ApiError> {
    let dir = repo_dir();
    let branch = git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
    let current = git(&dir, &["rev-parse", "HEAD"]).await?;

    // Fetch is best-effort: offline machines should still report current state.
    let _ = git(&dir, &["fetch", "--quiet"]).await;

    // Prefer the configured upstream; fall back to origin/<branch>.
    let upstream = match git(&dir, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]).await {
        Ok(u) => u,
        Err(_) => format!("origin/{branch}"),
    };

    let latest = git(&dir, &["rev-parse", &upstream])
        .await
        .unwrap_or_else(|_| current.clone());

    let counts = git(&dir, &["rev-list", "--left-right", "--count", &format!("HEAD...{upstream}")])
        .await
        .unwrap_or_else(|_| "0\t0".into());
    let mut it = counts.split_whitespace();
    let ahead: u32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let behind: u32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    let latest_subject = git(&dir, &["log", "-1", "--format=%s", &upstream])
        .await
        .unwrap_or_default();

    Ok(Json(UpdateStatus {
        branch,
        current_short: current.chars().take(7).collect(),
        current,
        latest_short: latest.chars().take(7).collect(),
        latest,
        behind,
        ahead,
        update_available: behind > 0,
        latest_subject,
        checked_at: chrono_now(),
    }))
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Serialize)]
pub struct ApplyResult {
    pub status: String,
    pub script: String,
    pub log: String,
}

async fn apply() -> Result<Json<ApplyResult>, ApiError> {
    let dir = repo_dir();
    let svc = service_name();
    let log_path = dir.join("data").join("update.log");
    let _ = std::fs::create_dir_all(dir.join("data"));

    let (script_path, status) = write_and_spawn(&dir, &svc, &log_path)?;

    Ok(Json(ApplyResult {
        status,
        script: script_path.to_string_lossy().into_owned(),
        log: log_path.to_string_lossy().into_owned(),
    }))
}

#[cfg(target_os = "windows")]
fn write_and_spawn(
    dir: &Path,
    svc: &str,
    log: &Path,
) -> Result<(PathBuf, String), ApiError> {
    use std::os::windows::process::CommandExt;

    // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB
    const FLAGS: u32 = 0x0000_0008 | 0x0000_0200 | 0x0100_0000;

    let repo = dir.to_string_lossy().replace('/', "\\");
    let logp = log.to_string_lossy().replace('/', "\\");
    let script = dir.join("data").join("helm-update.bat");
    let body = format!(
        "@echo off\r\n\
         setlocal\r\n\
         echo [%date% %time%] update start > \"{logp}\"\r\n\
         cd /d \"{repo}\"\r\n\
         echo stopping service... >> \"{logp}\"\r\n\
         nssm stop {svc} >> \"{logp}\" 2>&1\r\n\
         echo git pull... >> \"{logp}\"\r\n\
         git pull --ff-only >> \"{logp}\" 2>&1\r\n\
         echo building frontend... >> \"{logp}\"\r\n\
         cd client && call npm install >> \"{logp}\" 2>&1 && call npm run build >> \"{logp}\" 2>&1\r\n\
         cd /d \"{repo}\"\r\n\
         echo building backend... >> \"{logp}\"\r\n\
         cd helm-rs && cargo build --release >> \"{logp}\" 2>&1\r\n\
         cd /d \"{repo}\"\r\n\
         echo starting service... >> \"{logp}\"\r\n\
         nssm start {svc} >> \"{logp}\" 2>&1\r\n\
         echo [%date% %time%] update done >> \"{logp}\"\r\n"
    );
    std::fs::write(&script, body)
        .map_err(|e| ApiError::Internal(format!("write updater script: {e}")))?;

    // `cmd /c start` reparents the worker so an NSSM stop of HELM cannot reap it.
    std::process::Command::new("cmd")
        .args(["/c", "start", "HELM Update", "/min", "cmd", "/c"])
        .arg(&script)
        .current_dir(dir)
        .creation_flags(FLAGS)
        .spawn()
        .map_err(|e| ApiError::Internal(format!("spawn updater: {e}")))?;

    Ok((script, "started".into()))
}

#[cfg(not(target_os = "windows"))]
fn write_and_spawn(
    dir: &Path,
    svc: &str,
    log: &Path,
) -> Result<(PathBuf, String), ApiError> {
    use std::os::unix::fs::PermissionsExt;

    let repo = dir.to_string_lossy();
    let logp = log.to_string_lossy();
    let script = dir.join("data").join("helm-update.sh");
    let body = format!(
        "#!/usr/bin/env bash\n\
         set -e\n\
         exec >\"{logp}\" 2>&1\n\
         cd \"{repo}\"\n\
         echo 'git pull...'\n\
         git pull --ff-only\n\
         echo 'building frontend...'\n\
         (cd client && npm install && npm run build)\n\
         echo 'building backend...'\n\
         (cd helm-rs && cargo build --release)\n\
         echo 'restarting service...'\n\
         systemctl restart {svc} || sudo systemctl restart {svc}\n\
         echo 'update done'\n"
    );
    std::fs::write(&script, &body)
        .map_err(|e| ApiError::Internal(format!("write updater script: {e}")))?;
    let _ = std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755));

    // `setsid` detaches the worker from HELM's session so a restart won't kill it.
    std::process::Command::new("setsid")
        .arg("bash")
        .arg(&script)
        .current_dir(dir)
        .spawn()
        .map_err(|e| ApiError::Internal(format!("spawn updater: {e}")))?;

    Ok((script, "started".into()))
}
