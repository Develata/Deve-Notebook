//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-process-adapter-decision

use std::process::{Child, Command, Stdio};

use deve_core::native_adapter::{
    CURRENT_NATIVE_PROCESS_ADAPTER_POLICY, NativeEndpointReady, NativeProcessAdapterPolicy,
    NativeProcessExitStatus, NativeProcessRuntimeError, NativeProcessRuntimeEvent,
    NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle, NativeProcessRuntimeSnapshot,
    NativeProcessRuntimeState, NativeProcessSpawnSpec, NativeServiceHealthProbe,
};
use thiserror::Error;

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
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
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
        if self.core.snapshot.handle.is_some() {
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
        let exit_status = self.launcher.stop_service()?;
        Ok(match exit_status {
            Some(status) => self.core.record_stopped(Some(status), timestamp_unix_ms),
            None => self.core.record_stopped(None, timestamp_unix_ms),
        })
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
            Self::StopFailed { .. } => None,
        }
    }
}

fn validate_desktop_service_command(
    spec: &NativeProcessSpawnSpec,
) -> Result<(), DesktopProcessRuntimeError> {
    spec.validate_contract()?;
    let executable_name = spec
        .executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !matches!(executable_name, "deve_cli" | "deve_cli.exe") {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "executable must be deve_cli",
        });
    }
    if spec.argv.first().map(String::as_str) != Some("serve") {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "first argv must be serve",
        });
    }
    if spec.argv.len() != 4
        || spec.argv.get(1).map(String::as_str) != Some("--native-loopback")
        || spec.argv.get(2).map(String::as_str) != Some("--port")
    {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "argv must be exactly serve --native-loopback --port <port>",
        });
    }
    let Some(port) = spec.argv.get(3).and_then(|value| parse_nonzero_port(value)) else {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "argv port must be a non-zero u16",
        });
    };
    if spec.bind_hints.http_port != Some(port) || spec.bind_hints.ws_port != Some(port) {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "argv port must match loopback bind hints",
        });
    }
    Ok(())
}

fn parse_nonzero_port(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().filter(|port| *port != 0)
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

#[derive(Debug, Clone)]
struct DesktopProcessRuntimeCore {
    policy: NativeProcessAdapterPolicy,
    snapshot: NativeProcessRuntimeSnapshot,
    events: Vec<NativeProcessRuntimeEvent>,
    restart_attempt: u32,
    max_restart_attempts: u32,
}

impl DesktopProcessRuntimeCore {
    fn new(policy: NativeProcessAdapterPolicy, max_restart_attempts: u32) -> Self {
        Self {
            policy,
            snapshot: NativeProcessRuntimeSnapshot::disabled_by_policy(policy),
            events: Vec::new(),
            restart_attempt: 0,
            max_restart_attempts,
        }
    }

    fn request_start(
        &mut self,
        spec: &NativeProcessSpawnSpec,
        timestamp_unix_ms: i64,
    ) -> Result<NativeProcessRuntimeSnapshot, NativeProcessRuntimeError> {
        if !self.policy.child_process_runtime_enabled {
            return Err(NativeProcessRuntimeError::RuntimeDisabled);
        }
        spec.validate_contract()?;
        self.transition(NativeProcessRuntimeState::SpawnRequested, timestamp_unix_ms);
        Ok(self.snapshot())
    }

    fn record_started(
        &mut self,
        handle: NativeProcessRuntimeHandle,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.snapshot.handle = Some(handle);
        self.snapshot.started_at_unix_ms = Some(timestamp_unix_ms);
        self.transition(NativeProcessRuntimeState::Spawned, timestamp_unix_ms);
        self.snapshot()
    }

    fn record_endpoint_probe(
        &mut self,
        endpoint: NativeEndpointReady,
        probe: NativeServiceHealthProbe,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.snapshot.endpoint = Some(endpoint);
        self.snapshot.health_probe = probe;
        if probe.is_healthy() {
            self.transition(
                NativeProcessRuntimeState::EndpointHealthy,
                timestamp_unix_ms,
            );
        } else {
            self.record_failure(
                NativeProcessRuntimeFailureKind::HealthProbeFailed,
                timestamp_unix_ms,
            );
        }
        self.snapshot()
    }

    fn record_session_handoff(
        &mut self,
        session_bound: bool,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        if !session_bound {
            self.record_failure(
                NativeProcessRuntimeFailureKind::SessionHandoffFailed,
                timestamp_unix_ms,
            );
            return self.snapshot();
        }
        if let Some(endpoint) = self.snapshot.endpoint.as_mut() {
            endpoint.session_bound = true;
        }
        self.transition(
            NativeProcessRuntimeState::SessionHandoffReady,
            timestamp_unix_ms,
        );
        self.snapshot()
    }

    fn mark_runtime_ready(&mut self, timestamp_unix_ms: i64) -> NativeProcessRuntimeSnapshot {
        self.transition(NativeProcessRuntimeState::RuntimeReady, timestamp_unix_ms);
        self.snapshot()
    }

    fn record_process_exit(
        &mut self,
        status: NativeProcessExitStatus,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.snapshot.handle = None;
        self.snapshot.exit_status = Some(status);
        self.record_failure(
            NativeProcessRuntimeFailureKind::ProcessExited,
            timestamp_unix_ms,
        );
        self.snapshot()
    }

    fn record_stopped(
        &mut self,
        status: Option<NativeProcessExitStatus>,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.snapshot.handle = None;
        self.snapshot.exit_status = status;
        self.transition(NativeProcessRuntimeState::Stopped, timestamp_unix_ms);
        self.snapshot()
    }

    fn snapshot(&self) -> NativeProcessRuntimeSnapshot {
        self.snapshot.clone()
    }

    fn events(&self) -> &[NativeProcessRuntimeEvent] {
        &self.events
    }

    fn record_failure(&mut self, failure: NativeProcessRuntimeFailureKind, timestamp_unix_ms: i64) {
        let retryable =
            failure.retryable_by_default() && self.restart_attempt < self.max_restart_attempts;
        if retryable {
            self.restart_attempt += 1;
            self.transition(NativeProcessRuntimeState::Restarting, timestamp_unix_ms);
        } else {
            self.transition(NativeProcessRuntimeState::Offline, timestamp_unix_ms);
        }
        self.snapshot.last_failure = Some(failure);
        if let Some(event) = self.events.last_mut() {
            event.failure = Some(failure);
        }
    }

    fn transition(&mut self, state: NativeProcessRuntimeState, timestamp_unix_ms: i64) {
        self.snapshot.state = state;
        self.events.push(NativeProcessRuntimeEvent {
            state,
            timestamp_unix_ms,
            failure: None,
        });
    }
}
