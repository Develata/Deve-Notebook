//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-process-adapter-decision

use std::process::{Child, Command, Stdio};

use deve_core::native_adapter::{
    NativeProcessExitStatus, NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle,
    NativeProcessSpawnSpec,
};

use super::validation::validate_desktop_service_command;
use super::{DesktopProcessLauncher, DesktopProcessRuntimeError};

const DEVE_DESKTOP_SERVICE_STDIO_INHERIT_ENV: &str = "DEVE_DESKTOP_SERVICE_STDIO_INHERIT";

#[derive(Debug, Default)]
pub struct DesktopCommandProcessLauncher {
    child: Option<Child>,
}

impl DesktopCommandProcessLauncher {
    pub fn stop(&mut self) -> std::io::Result<Option<NativeProcessExitStatus>> {
        let Some(mut child) = self.child.take() else {
            return Ok(None);
        };
        let _ = child.kill();
        let status = child.wait()?;
        Ok(Some(exit_status_from_process_status(status)))
    }
}

impl Drop for DesktopCommandProcessLauncher {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl DesktopProcessLauncher for DesktopCommandProcessLauncher {
    fn spawn_service(
        &mut self,
        spec: &NativeProcessSpawnSpec,
    ) -> Result<NativeProcessRuntimeHandle, DesktopProcessRuntimeError> {
        if self.child.is_some() {
            return Err(DesktopProcessRuntimeError::AlreadyRunning);
        }
        validate_desktop_service_command(spec)?;

        let mut command = Command::new(&spec.executable);
        command
            .args(&spec.argv)
            .current_dir(&spec.cwd)
            .env_clear()
            .stdin(Stdio::null());
        if std::env::var_os(DEVE_DESKTOP_SERVICE_STDIO_INHERIT_ENV).is_some() {
            command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        } else {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        for binding in &spec.env {
            command.env(&binding.key, &binding.value);
        }

        let child = command
            .spawn()
            .map_err(|source| DesktopProcessRuntimeError::SpawnFailed {
                kind: spawn_failure_kind(&source),
                source,
            })?;
        let pid = child.id();
        self.child = Some(child);
        Ok(NativeProcessRuntimeHandle {
            handle_id: format!("desktop-service-{pid}"),
            platform_pid: Some(pid),
        })
    }

    fn stop_service(
        &mut self,
    ) -> Result<Option<NativeProcessExitStatus>, DesktopProcessRuntimeError> {
        self.stop()
            .map_err(|source| DesktopProcessRuntimeError::StopFailed { source })
    }
}

fn spawn_failure_kind(error: &std::io::Error) -> NativeProcessRuntimeFailureKind {
    match error.kind() {
        std::io::ErrorKind::NotFound => NativeProcessRuntimeFailureKind::SpawnExecutableMissing,
        std::io::ErrorKind::PermissionDenied => {
            NativeProcessRuntimeFailureKind::SpawnPermissionDenied
        }
        _ => NativeProcessRuntimeFailureKind::InvalidExecutablePath,
    }
}

fn exit_status_from_process_status(status: std::process::ExitStatus) -> NativeProcessExitStatus {
    NativeProcessExitStatus {
        code: status.code(),
        signal: exit_signal(status),
    }
}

#[cfg(unix)]
fn exit_signal(status: std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;

    status.signal()
}

#[cfg(not(unix))]
fn exit_signal(_status: std::process::ExitStatus) -> Option<i32> {
    None
}
