use deve_core::native_adapter::{
    NativeProcessExitStatus, NativeProcessRuntimeError, NativeProcessRuntimeFailureKind,
    NativeProcessRuntimeState, NativeServiceHealthProbe,
};

use crate::process_runtime::{DesktopLocalServiceRuntime, DesktopProcessRuntimeError};

use super::support::{
    RecordingLauncher, closed_policy, enabled_policy, endpoint, handle, healthy_probe,
    spawn_missing_error, valid_spawn_spec,
};

#[test]
fn desktop_local_service_runtime_blocks_when_policy_closed() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(closed_policy(), 2, launcher);

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
fn desktop_local_service_runtime_records_spawn_failure_without_authority() {
    let launcher = RecordingLauncher::with_error(spawn_missing_error());
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
