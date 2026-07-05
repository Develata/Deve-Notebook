//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

use deve_core::native_adapter::{
    NativeProcessExitStatus, NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle,
    NativeProcessSpawnSpec,
};

use super::process_group::{
    DesktopChildProcess, DesktopChildProcessGroup, DesktopChildProcessSpawnError,
};
use super::validation::validate_desktop_service_command;
use super::{DesktopProcessLauncher, DesktopProcessRuntimeError};
use std::time::Duration;

const DEVE_DESKTOP_SERVICE_STDIO_INHERIT_ENV: &str = "DEVE_DESKTOP_SERVICE_STDIO_INHERIT";
const DESKTOP_SERVICE_STOP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Default)]
pub struct DesktopCommandProcessLauncher {
    child: Option<DesktopChildProcess>,
    process_group: Option<DesktopChildProcessGroup>,
}

impl DesktopCommandProcessLauncher {
    pub fn stop(&mut self) -> std::io::Result<Option<NativeProcessExitStatus>> {
        let Some(mut child) = self.child.take() else {
            return Ok(None);
        };
        let process_group = self.process_group.take();
        let _ = child.kill();
        drop(process_group);
        let status = child.wait_timeout(DESKTOP_SERVICE_STOP_TIMEOUT);
        if status.is_err() {
            let _ = child.kill();
        }
        status.map(Some)
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

        let inherit_stdio = std::env::var_os(DEVE_DESKTOP_SERVICE_STDIO_INHERIT_ENV).is_some();
        let process_group = DesktopChildProcessGroup::new()
            .map_err(|source| DesktopProcessRuntimeError::ContainmentFailed { source })?;
        let child = process_group
            .spawn_service(spec, inherit_stdio)
            .map_err(runtime_error_from_spawn_error)?;
        let pid = child.id();
        self.process_group = Some(process_group);
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

fn runtime_error_from_spawn_error(
    error: DesktopChildProcessSpawnError,
) -> DesktopProcessRuntimeError {
    match error {
        DesktopChildProcessSpawnError::SpawnFailed(source) => {
            DesktopProcessRuntimeError::SpawnFailed {
                kind: spawn_failure_kind(&source),
                source,
            }
        }
        DesktopChildProcessSpawnError::ContainmentFailed(source) => {
            DesktopProcessRuntimeError::ContainmentFailed { source }
        }
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
