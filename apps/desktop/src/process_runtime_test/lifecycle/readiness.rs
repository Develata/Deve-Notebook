use deve_core::native_adapter::{NativeProcessRuntimeFailureKind, NativeProcessRuntimeState};

use crate::process_runtime::DesktopLocalServiceRuntime;

use super::super::support::{
    RecordingLauncher, enabled_policy, endpoint, handle, healthy_probe, valid_spawn_spec,
};

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
