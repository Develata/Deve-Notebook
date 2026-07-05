//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

use deve_core::native_adapter::{NativeProcessExitStatus, NativeProcessSpawnSpec};

#[cfg(not(windows))]
use std::process::{Child, Command, Stdio};
use std::time::Duration;
#[cfg(not(windows))]
use std::time::Instant;

#[cfg(windows)]
mod windows;

#[derive(Debug)]
pub(super) struct DesktopChildProcessGroup {
    #[cfg(windows)]
    job: windows::KillOnCloseJob,
}

#[derive(Debug)]
pub(super) struct DesktopChildProcess {
    #[cfg(windows)]
    process: windows::JobChildProcess,
    #[cfg(not(windows))]
    child: Child,
}

#[derive(Debug)]
pub(super) enum DesktopChildProcessSpawnError {
    SpawnFailed(std::io::Error),
    ContainmentFailed(std::io::Error),
}

impl DesktopChildProcessGroup {
    pub(super) fn new() -> std::io::Result<Self> {
        Ok(Self {
            #[cfg(windows)]
            job: windows::KillOnCloseJob::new()?,
        })
    }

    pub(super) fn spawn_service(
        &self,
        spec: &NativeProcessSpawnSpec,
        inherit_stdio: bool,
    ) -> Result<DesktopChildProcess, DesktopChildProcessSpawnError> {
        #[cfg(windows)]
        {
            self.job
                .spawn_service(spec, inherit_stdio)
                .map(|process| DesktopChildProcess { process })
        }
        #[cfg(not(windows))]
        {
            let _ = self;
            let mut command = Command::new(&spec.executable);
            command
                .args(&spec.argv)
                .current_dir(&spec.cwd)
                .env_clear()
                .stdin(Stdio::null());
            if inherit_stdio {
                command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
            } else {
                command.stdout(Stdio::null()).stderr(Stdio::null());
            }
            for binding in &spec.env {
                command.env(&binding.key, &binding.value);
            }

            command
                .spawn()
                .map(|child| DesktopChildProcess { child })
                .map_err(DesktopChildProcessSpawnError::SpawnFailed)
        }
    }
}

impl DesktopChildProcess {
    pub(super) fn id(&self) -> u32 {
        #[cfg(windows)]
        {
            self.process.id()
        }
        #[cfg(not(windows))]
        {
            self.child.id()
        }
    }

    pub(super) fn kill(&mut self) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            self.process.kill()
        }
        #[cfg(not(windows))]
        {
            self.child.kill()
        }
    }

    pub(super) fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<NativeProcessExitStatus> {
        #[cfg(windows)]
        {
            self.process.wait_timeout(timeout)
        }
        #[cfg(not(windows))]
        {
            let deadline = Instant::now() + timeout;
            loop {
                if let Some(status) = self.child.try_wait()? {
                    return Ok(exit_status_from_process_status(status));
                }
                if Instant::now() >= deadline {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "desktop local service did not exit before stop timeout",
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

#[cfg(not(windows))]
fn exit_status_from_process_status(status: std::process::ExitStatus) -> NativeProcessExitStatus {
    NativeProcessExitStatus {
        code: status.code(),
        signal: exit_signal(status),
    }
}

#[cfg(all(not(windows), unix))]
fn exit_signal(status: std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;

    status.signal()
}

#[cfg(all(not(windows), not(unix)))]
fn exit_signal(_status: std::process::ExitStatus) -> Option<i32> {
    None
}
