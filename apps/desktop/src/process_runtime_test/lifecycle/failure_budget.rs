use deve_core::native_adapter::{
    NativeProcessExitStatus, NativeProcessRuntimeFailureKind, NativeProcessRuntimeState,
    NativeServiceHealthProbe,
};

use crate::process_runtime::{DesktopLocalServiceRuntime, DesktopProcessRuntimeError};

use super::super::support::{
    RecordingLauncher, containment_error, enabled_policy, endpoint, handle, spawn_missing_error,
    valid_spawn_spec,
};

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
fn desktop_local_service_runtime_records_containment_failure_without_authority() {
    let launcher = RecordingLauncher::with_error(containment_error());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);

    let error = runtime
        .start(&valid_spawn_spec(), 10)
        .expect_err("containment failure");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::ContainmentFailed { .. }
    ));
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.state, NativeProcessRuntimeState::Offline);
    assert_eq!(
        snapshot.last_failure,
        Some(NativeProcessRuntimeFailureKind::ProcessContainmentFailed)
    );
    assert!(!snapshot.authority_writes_allowed);
}
