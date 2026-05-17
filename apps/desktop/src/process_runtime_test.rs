use std::path::PathBuf;

use deve_core::native_adapter::{
    NativeEndpointReady, NativeProcessAdapterDecision, NativeProcessAdapterPolicy,
    NativeProcessBindHints, NativeProcessEnvBinding, NativeProcessExitStatus,
    NativeProcessPathResolution, NativeProcessRuntimeError, NativeProcessRuntimeFailureKind,
    NativeProcessRuntimeHandle, NativeProcessRuntimeState, NativeProcessSpawnSpec,
    NativeRuntimeReadiness, NativeServiceHealthProbe,
};

use crate::process_runtime::{
    DesktopLocalServiceRuntime, DesktopProcessLauncher, DesktopProcessRuntimeError,
};
use crate::{DesktopServiceState, DesktopSessionMaterial, DesktopShell};

fn valid_spawn_spec() -> NativeProcessSpawnSpec {
    let root = std::env::current_dir().expect("current dir");
    NativeProcessSpawnSpec {
        executable: root.join("target/native/deve_cli"),
        argv: vec!["serve".to_string(), "--dev".to_string()],
        cwd: root.clone(),
        env_allowlist: vec!["DEVE_PROFILE".to_string()],
        env: vec![NativeProcessEnvBinding {
            key: "DEVE_PROFILE".to_string(),
            value: "standard".to_string(),
        }],
        profile: "standard".to_string(),
        config_path: root.join("config.toml"),
        vault_path: root.join("vault"),
        ledger_path: root.join("ledger"),
        bind_hints: NativeProcessBindHints {
            http_host: "127.0.0.1".to_string(),
            http_port: Some(3001),
            ws_host: "localhost".to_string(),
            ws_port: Some(3001),
        },
        path_resolution: NativeProcessPathResolution::AbsoluteOnly,
    }
}

fn handle() -> NativeProcessRuntimeHandle {
    NativeProcessRuntimeHandle {
        handle_id: "fake-child-1".to_string(),
        platform_pid: Some(4242),
    }
}

#[derive(Debug, Default)]
struct RecordingLauncher {
    calls: usize,
    result: Option<Result<NativeProcessRuntimeHandle, DesktopProcessRuntimeError>>,
    stop_calls: usize,
    stop_result: Option<Result<Option<NativeProcessExitStatus>, DesktopProcessRuntimeError>>,
}

impl RecordingLauncher {
    fn with_handle(handle: NativeProcessRuntimeHandle) -> Self {
        Self {
            calls: 0,
            result: Some(Ok(handle)),
            stop_calls: 0,
            stop_result: Some(Ok(Some(NativeProcessExitStatus {
                code: Some(0),
                signal: None,
            }))),
        }
    }

    fn with_error(error: DesktopProcessRuntimeError) -> Self {
        Self {
            calls: 0,
            result: Some(Err(error)),
            stop_calls: 0,
            stop_result: Some(Ok(None)),
        }
    }
}

impl DesktopProcessLauncher for RecordingLauncher {
    fn spawn_service(
        &mut self,
        _spec: &NativeProcessSpawnSpec,
    ) -> Result<NativeProcessRuntimeHandle, DesktopProcessRuntimeError> {
        self.calls += 1;
        self.result.take().unwrap_or_else(|| {
            Ok(NativeProcessRuntimeHandle {
                handle_id: format!("fake-child-{}", self.calls),
                platform_pid: Some(self.calls as u32),
            })
        })
    }

    fn stop_service(
        &mut self,
    ) -> Result<Option<NativeProcessExitStatus>, DesktopProcessRuntimeError> {
        self.stop_calls += 1;
        self.stop_result.take().unwrap_or(Ok(None))
    }
}

fn enabled_policy() -> NativeProcessAdapterPolicy {
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
        child_process_runtime_enabled: true,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    }
}

fn endpoint(session_bound: bool) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound,
    }
}

fn healthy_probe() -> NativeServiceHealthProbe {
    NativeServiceHealthProbe {
        endpoint_reachable: true,
        node_role_readable: true,
    }
}

#[test]
fn desktop_local_service_runtime_blocks_when_policy_closed() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(
        NativeProcessAdapterPolicy {
            decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
            child_process_runtime_enabled: false,
            packaging_gate_required: true,
            authority_writes_allowed: false,
        },
        2,
        launcher,
    );

    let error = runtime
        .start(&valid_spawn_spec(), 1)
        .expect_err("runtime disabled");
    assert!(matches!(
        error,
        DesktopProcessRuntimeError::Contract(NativeProcessRuntimeError::RuntimeDisabled)
    ));
    assert_eq!(
        runtime.snapshot().state,
        NativeProcessRuntimeState::Disabled
    );
    assert!(runtime.events().is_empty());
}

#[test]
fn desktop_local_service_runtime_rejects_invalid_spawn_spec() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    let mut spec = valid_spawn_spec();
    spec.executable = PathBuf::from("deve_cli");

    let error = runtime.start(&spec, 1).expect_err("invalid spawn spec");
    assert!(matches!(
        error,
        DesktopProcessRuntimeError::Contract(NativeProcessRuntimeError::RelativePathForbidden {
            field: "executable"
        })
    ));
    assert_eq!(
        runtime.snapshot().state,
        NativeProcessRuntimeState::Disabled
    );
    assert!(runtime.events().is_empty());
}

#[test]
fn desktop_local_service_runtime_records_successful_state_sequence() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);

    runtime
        .start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_endpoint_probe(endpoint(false), healthy_probe(), 2);
    runtime.record_session_handoff(true, 3);
    runtime.mark_runtime_ready(4);

    let states: Vec<_> = runtime.events().iter().map(|event| event.state).collect();
    assert_eq!(
        states,
        [
            NativeProcessRuntimeState::SpawnRequested,
            NativeProcessRuntimeState::Spawned,
            NativeProcessRuntimeState::EndpointHealthy,
            NativeProcessRuntimeState::SessionHandoffReady,
            NativeProcessRuntimeState::RuntimeReady,
        ]
    );
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.state, NativeProcessRuntimeState::RuntimeReady);
    assert!(snapshot.handle.is_some());
    assert!(snapshot.endpoint.expect("endpoint").session_bound);
}

#[test]
fn desktop_local_service_runtime_probe_failure_consumes_retry_budget() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    runtime
        .start(&valid_spawn_spec(), 1)
        .expect("request start");

    let retry =
        runtime.record_endpoint_probe(endpoint(false), NativeServiceHealthProbe::default(), 2);
    assert_eq!(retry.state, NativeProcessRuntimeState::Restarting);
    assert_eq!(
        retry.last_failure,
        Some(NativeProcessRuntimeFailureKind::HealthProbeFailed)
    );

    let terminal =
        runtime.record_endpoint_probe(endpoint(false), NativeServiceHealthProbe::default(), 3);
    assert_eq!(terminal.state, NativeProcessRuntimeState::Offline);
    assert_eq!(
        terminal.last_failure,
        Some(NativeProcessRuntimeFailureKind::HealthProbeFailed)
    );
}

#[test]
fn desktop_local_service_runtime_process_exit_consumes_retry_budget() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    runtime
        .start(&valid_spawn_spec(), 1)
        .expect("request start");

    let retry = runtime.record_process_exit(
        NativeProcessExitStatus {
            code: Some(1),
            signal: None,
        },
        3,
    );
    assert_eq!(retry.state, NativeProcessRuntimeState::Restarting);
    assert!(retry.handle.is_none());
    assert_eq!(
        retry.last_failure,
        Some(NativeProcessRuntimeFailureKind::ProcessExited)
    );

    let terminal = runtime.record_process_exit(
        NativeProcessExitStatus {
            code: Some(1),
            signal: None,
        },
        4,
    );
    assert_eq!(terminal.state, NativeProcessRuntimeState::Offline);
}

#[test]
fn desktop_local_service_runtime_session_handoff_failure_is_fatal() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 3, launcher);
    runtime
        .start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_endpoint_probe(endpoint(false), healthy_probe(), 2);

    let snapshot = runtime.record_session_handoff(false, 3);

    assert_eq!(snapshot.state, NativeProcessRuntimeState::Offline);
    assert_eq!(
        snapshot.last_failure,
        Some(NativeProcessRuntimeFailureKind::SessionHandoffFailed)
    );
}

#[test]
fn desktop_local_service_runtime_does_not_unlock_writable_shell_without_writer_gate() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    runtime
        .start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_endpoint_probe(endpoint(false), healthy_probe(), 2);
    runtime.record_session_handoff(true, 3);
    runtime.mark_runtime_ready(4);

    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint(false)).expect("bind endpoint");
    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session");

    let readiness = NativeRuntimeReadiness {
        endpoint_reachable: true,
        auth_status_valid: true,
        node_role_readable: true,
        repo_handshake_complete: true,
        writer_ready: false,
        scope_nonce_current: true,
    };

    assert!(!shell.mark_runtime_ready(readiness));
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionBound);
}

#[test]
fn desktop_local_service_runtime_starts_valid_deve_cli_serve_spec() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);

    let snapshot = runtime
        .start(&valid_spawn_spec(), 10)
        .expect("start local service");

    assert_eq!(snapshot.state, NativeProcessRuntimeState::Spawned);
    assert_eq!(snapshot.handle.expect("handle").platform_pid, Some(4242));
    assert!(!snapshot.authority_writes_allowed);
    assert_eq!(
        runtime
            .events()
            .iter()
            .map(|event| event.state)
            .collect::<Vec<_>>(),
        [
            NativeProcessRuntimeState::SpawnRequested,
            NativeProcessRuntimeState::Spawned,
        ]
    );
}

#[test]
fn desktop_local_service_runtime_rejects_second_start_without_stopping() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    runtime
        .start(&valid_spawn_spec(), 10)
        .expect("start local service");

    let error = runtime
        .start(&valid_spawn_spec(), 11)
        .expect_err("reject second start");

    assert!(matches!(error, DesktopProcessRuntimeError::AlreadyRunning));
    assert_eq!(runtime.snapshot().state, NativeProcessRuntimeState::Spawned);
    assert!(runtime.snapshot().handle.is_some());
}

#[test]
fn desktop_local_service_runtime_rejects_non_deve_cli_command_before_spawn() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    let mut spec = valid_spawn_spec();
    spec.executable = std::env::current_dir()
        .expect("current dir")
        .join("target/native/other_tool");

    let error = runtime.start(&spec, 10).expect_err("reject command");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "executable must be deve_cli"
        }
    ));
    assert_eq!(runtime.snapshot().state, NativeProcessRuntimeState::Offline);
    assert_eq!(
        runtime.snapshot().last_failure,
        Some(NativeProcessRuntimeFailureKind::InvalidExecutablePath)
    );
}

#[test]
fn desktop_local_service_runtime_rejects_non_serve_argv_before_spawn() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    let mut spec = valid_spawn_spec();
    spec.argv = vec!["dump".to_string()];

    let error = runtime.start(&spec, 10).expect_err("reject argv");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "first argv must be serve"
        }
    ));
}

#[test]
fn desktop_local_service_runtime_records_spawn_failure_without_authority() {
    let launcher = RecordingLauncher::with_error(DesktopProcessRuntimeError::SpawnFailed {
        kind: NativeProcessRuntimeFailureKind::SpawnExecutableMissing,
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
    });
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);

    let error = runtime
        .start(&valid_spawn_spec(), 10)
        .expect_err("spawn failure");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::SpawnFailed {
            kind: NativeProcessRuntimeFailureKind::SpawnExecutableMissing,
            ..
        }
    ));
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.state, NativeProcessRuntimeState::Offline);
    assert_eq!(
        snapshot.last_failure,
        Some(NativeProcessRuntimeFailureKind::SpawnExecutableMissing)
    );
    assert!(!snapshot.authority_writes_allowed);
}

#[test]
fn desktop_local_service_runtime_stops_started_child_without_authority() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    runtime
        .start(&valid_spawn_spec(), 10)
        .expect("start local service");

    let snapshot = runtime.stop(20).expect("stop local service");

    assert_eq!(snapshot.state, NativeProcessRuntimeState::Stopped);
    assert!(snapshot.handle.is_none());
    assert_eq!(
        snapshot.exit_status,
        Some(NativeProcessExitStatus {
            code: Some(0),
            signal: None
        })
    );
    assert!(!snapshot.authority_writes_allowed);
    assert_eq!(
        runtime
            .events()
            .iter()
            .map(|event| event.state)
            .collect::<Vec<_>>(),
        [
            NativeProcessRuntimeState::SpawnRequested,
            NativeProcessRuntimeState::Spawned,
            NativeProcessRuntimeState::Stopped,
        ]
    );
}
