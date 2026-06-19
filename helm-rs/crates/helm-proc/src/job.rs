//! Cross-platform process-tree termination.
//!
//! - **Windows**: wrap the spawned child in a JobObject with KILL_ON_JOB_CLOSE so
//!   tearing down the handle wipes the entire descendant tree. `taskkill /F /T`
//!   is the fallback when JobObject assignment fails.
//! - **Unix**: child runs in its own session (via `setsid` in `pre_exec`) and is
//!   killed with `killpg`.

#[cfg(windows)]
pub use win::{taskkill_tree, JobHandle};

#[cfg(unix)]
pub use unix::{kill_process_group, set_session_leader};

#[cfg(windows)]
mod win {
    use anyhow::{anyhow, Result};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    pub struct JobHandle {
        handle: HANDLE,
    }

    // The HANDLE is owned by this struct and only touched through Win32 calls
    // that are safe to invoke from any thread.
    unsafe impl Send for JobHandle {}
    unsafe impl Sync for JobHandle {}

    impl JobHandle {
        /// Create a kill-on-close JobObject and assign `pid` to it.
        pub fn create_and_assign(pid: u32) -> Result<Self> {
            unsafe {
                let handle = CreateJobObjectW(None, windows::core::PCWSTR::null())
                    .map_err(|e| anyhow!("CreateJobObjectW failed: {e}"))?;

                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .map_err(|e| {
                    let _ = CloseHandle(handle);
                    anyhow!("SetInformationJobObject failed: {e}")
                })?;

                let proc = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid).map_err(
                    |e| {
                        let _ = CloseHandle(handle);
                        anyhow!("OpenProcess({pid}) failed: {e}")
                    },
                )?;

                let assign = AssignProcessToJobObject(handle, proc);
                let _ = CloseHandle(proc);
                assign.map_err(|e| {
                    let _ = CloseHandle(handle);
                    anyhow!("AssignProcessToJobObject failed: {e}")
                })?;

                Ok(Self { handle })
            }
        }

        /// Terminate the entire job (all descendant processes).
        pub fn terminate(&self) -> Result<()> {
            unsafe {
                TerminateJobObject(self.handle, 1)
                    .map_err(|e| anyhow!("TerminateJobObject failed: {e}"))?;
            }
            Ok(())
        }
    }

    impl Drop for JobHandle {
        fn drop(&mut self) {
            // KILL_ON_JOB_CLOSE means closing the last handle also kills children.
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }

    /// `taskkill /F /T /PID <pid>` fallback. Returns true if exit code 0.
    pub async fn taskkill_tree(pid: u32) -> bool {
        let res = tokio::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        matches!(res, Ok(s) if s.success())
    }
}

#[cfg(unix)]
mod unix {
    use nix::sys::signal::{killpg, Signal};
    use nix::unistd::Pid;

    /// Pre-exec hook to make the child a new session leader (`setsid`). Call this
    /// from `std::os::unix::process::CommandExt::pre_exec`.
    pub fn set_session_leader() -> std::io::Result<()> {
        // SAFETY: setsid is async-signal-safe and safe to call between fork/exec.
        unsafe {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Send a signal to the entire process group rooted at `pid`.
    pub fn kill_process_group(pid: u32, sig: Signal) -> nix::Result<()> {
        killpg(Pid::from_raw(pid as i32), sig)
    }
}
