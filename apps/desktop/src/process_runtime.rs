//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

mod core;
mod launcher;
mod process_group;
mod validation;

use deve_core::native_adapter::{
    CURRENT_NATIVE_PROCESS_ADAPTER_POLICY, NativeEndpointReady, NativeProcessAdapterPolicy,
    NativeProcessExitStatus, NativeProcessRuntimeError, NativeProcessRuntimeEvent,
    NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle, NativeProcessRuntimeSnapshot,
    NativeProcessSpawnSpec, NativeServiceHealthProbe,
};
use thiserror::Error;

use core::DesktopProcessRuntimeCore;
pub use launcher::DesktopCommandProcessLauncher;
use validation::validate_desktop_service_command;

#[derive(Debug, Error)]
pub enum DesktopProcessRuntimeError {
    #[error(transparent)]
    Contract(#[from] NativeProcessRuntimeError),
    #[error("desktop local service is already running")]
    AlreadyRunning,
    #[error("desktop local service runtime requires deve_cli serve: {reason}")]
    InvalidServiceCommand { reason: &'static str },
    #[error("failed to spawn desktop local service")]
    SpawnFailed {
        kind: NativeProcessRuntimeFailureKind,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to bind desktop local service to parent process lifetime")]
    ContainmentFailed {
        #[source]
        source: std::io::Error,
    },
    #[error("failed to stop desktop local service")]
    StopFailed {
        #[source]
        source: std::io::Error,
    },
}

pub trait DesktopProcessLauncher {
    fn spawn_service(
        &mut self,
        spec: &NativeProcessSpawnSpec,
    ) -> Result<NativeProcessRuntimeHandle, DesktopProcessRuntimeError>;

    fn stop_service(
        &mut self,
    ) -> Result<Option<NativeProcessExitStatus>, DesktopProcessRuntimeError>;
}

#[derive(Debug)]
pub struct DesktopLocalServiceRuntime<L = DesktopCommandProcessLauncher> {
    core: DesktopProcessRuntimeCore,
    launcher: L,
}

impl DesktopLocalServiceRuntime<DesktopCommandProcessLauncher> {
    pub fn disabled() -> Self {
        Self::with_launcher(
            CURRENT_NATIVE_PROCESS_ADAPTER_POLICY,
            2,
            DesktopCommandProcessLauncher::default(),
        )
    }
}

impl<L: DesktopProcessLauncher> DesktopLocalServiceRuntime<L> {
    pub fn with_launcher(
        policy: NativeProcessAdapterPolicy,
        max_restart_attempts: u32,
        launcher: L,
    ) -> Self {
        Self {
            core: DesktopProcessRuntimeCore::new(policy, max_restart_attempts),
            launcher,
        }
    }

    pub fn start(
        &mut self,
        spec: &NativeProcessSpawnSpec,
        timestamp_unix_ms: i64,
    ) -> Result<NativeProcessRuntimeSnapshot, DesktopProcessRuntimeError> {
        if self.core.is_running() {
            return Err(DesktopProcessRuntimeError::AlreadyRunning);
        }
        self.core.request_start(spec, timestamp_unix_ms)?;
        if let Err(error) = validate_desktop_service_command(spec) {
            if let Some(kind) = error.failure_kind() {
                self.core.record_failure(kind, timestamp_unix_ms);
            }
            return Err(error);
        }
        match self.launcher.spawn_service(spec) {
            Ok(handle) => Ok(self.core.record_started(handle, timestamp_unix_ms)),
            Err(error) => {
                if let Some(kind) = error.failure_kind() {
                    self.core.record_failure(kind, timestamp_unix_ms);
                }
                Err(error)
            }
        }
    }

    pub fn record_endpoint_probe(
        &mut self,
        endpoint: NativeEndpointReady,
        probe: NativeServiceHealthProbe,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.core
            .record_endpoint_probe(endpoint, probe, timestamp_unix_ms)
    }

    pub fn record_health_probe_failure(
        &mut self,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.core.record_failure(
            NativeProcessRuntimeFailureKind::HealthProbeFailed,
            timestamp_unix_ms,
        );
        self.core.snapshot()
    }

    pub fn record_session_handoff(
        &mut self,
        session_bound: bool,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.core
            .record_session_handoff(session_bound, timestamp_unix_ms)
    }

    pub fn mark_runtime_ready(&mut self, timestamp_unix_ms: i64) -> NativeProcessRuntimeSnapshot {
        self.core.mark_runtime_ready(timestamp_unix_ms)
    }

    pub fn record_process_exit(
        &mut self,
        status: NativeProcessExitStatus,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.core.record_process_exit(status, timestamp_unix_ms)
    }

    pub fn stop(
        &mut self,
        timestamp_unix_ms: i64,
    ) -> Result<NativeProcessRuntimeSnapshot, DesktopProcessRuntimeError> {
        match self.launcher.stop_service() {
            Ok(Some(status)) => Ok(self.core.record_stopped(Some(status), timestamp_unix_ms)),
            Ok(None) => Ok(self.core.record_stopped(None, timestamp_unix_ms)),
            Err(error) => {
                self.core.record_stopped(None, timestamp_unix_ms);
                Err(error)
            }
        }
    }

    pub fn snapshot(&self) -> NativeProcessRuntimeSnapshot {
        self.core.snapshot()
    }

    pub fn events(&self) -> &[NativeProcessRuntimeEvent] {
        self.core.events()
    }
}

impl DesktopProcessRuntimeError {
    fn failure_kind(&self) -> Option<NativeProcessRuntimeFailureKind> {
        match self {
            Self::Contract(_) => None,
            Self::AlreadyRunning => None,
            Self::InvalidServiceCommand { .. } => {
                Some(NativeProcessRuntimeFailureKind::InvalidExecutablePath)
            }
            Self::SpawnFailed { kind, .. } => Some(*kind),
            Self::ContainmentFailed { .. } => {
                Some(NativeProcessRuntimeFailureKind::ProcessContainmentFailed)
            }
            Self::StopFailed { .. } => None,
        }
    }
}
