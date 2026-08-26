use deve_core::native_adapter::{
    NativeProcessExitStatus, NativeProcessRuntimeError, NativeProcessRuntimeState,
};

use crate::process_runtime::{DesktopLocalServiceRuntime, DesktopProcessRuntimeError};

use super::super::support::{
    RecordingLauncher, closed_policy, enabled_policy, handle, valid_spawn_spec,
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

#[test]
fn desktop_local_service_runtime_stop_failure_still_releases_controlled_handle() {
    let launcher = RecordingLauncher::with_handle_and_stop_error(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    runtime
        .start(&valid_spawn_spec(), 10)
        .expect("start local service");

    let error = runtime.stop(20).expect_err("stop failure is reported");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::StopFailed { .. }
    ));
    assert_eq!(runtime.snapshot().state, NativeProcessRuntimeState::Stopped);
    assert!(runtime.snapshot().handle.is_none());
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

#[test]
fn desktop_process_runtime_event_history_is_bounded() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);

    for timestamp in 0..100 {
        runtime.record_health_probe_failure(timestamp);
    }

    assert_eq!(runtime.events().len(), 64);
    assert_eq!(
        runtime.events().first().expect("oldest").timestamp_unix_ms,
        36
    );
    assert_eq!(
        runtime.events().last().expect("newest").timestamp_unix_ms,
        99
    );
    assert_eq!(runtime.snapshot().state, NativeProcessRuntimeState::Offline);
}
