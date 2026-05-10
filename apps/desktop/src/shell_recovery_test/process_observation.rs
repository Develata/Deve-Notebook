use super::support::{bound_shell, endpoint, ready_probe};
use crate::{DesktopServiceState, DesktopSessionMaterial, DesktopShellError};
use deve_core::native_adapter::{
    NativeProcessAdapterState, NativeRuntimeReadiness, NativeServiceRestarting,
    NativeServiceSupervisorState,
};

#[test]
fn desktop_probe_timeout_observation_uses_process_snapshot() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_probe_timeout(),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "probe_failed"
    ));

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceRestarting);
    assert_eq!(snapshot.readiness, NativeRuntimeReadiness::default());
    assert_eq!(
        snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 1 })
    );
    assert!(snapshot.endpoint.is_none());
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Restarting
    );
    assert_eq!(snapshot.supervisor.restart_attempt, 1);
    assert!(!snapshot.process_adapter.health_probe.is_healthy());
    assert_eq!(
        snapshot.process_adapter.state,
        NativeProcessAdapterState::SessionHandoffReady
    );
}

#[test]
fn desktop_probe_timeout_requires_endpoint_rebind_before_session_handoff() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_probe_timeout(),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "probe_failed"
    ));
    let before_failed_session = shell.snapshot();
    assert!(matches!(
        shell.bind_session(DesktopSessionMaterial::bound()),
        Err(DesktopShellError::SessionNotBound)
    ));
    let after_failed_session = shell.snapshot();
    assert_eq!(
        after_failed_session.process_adapter,
        before_failed_session.process_adapter
    );
    assert_eq!(
        after_failed_session.supervisor,
        before_failed_session.supervisor
    );
    let blocked = shell.snapshot();
    assert_eq!(blocked.state, DesktopServiceState::ServiceRestarting);
    assert_eq!(blocked.supervisor.restart_attempt, 1);
    assert!(blocked.endpoint.is_none());

    shell.bind_endpoint(endpoint()).expect("rebind endpoint");
    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session after endpoint rebind");

    let recovered = shell.snapshot();
    assert_eq!(recovered.state, DesktopServiceState::SessionBound);
    assert_eq!(
        recovered.supervisor.state,
        NativeServiceSupervisorState::SessionHandoffReady
    );
    assert!(recovered.endpoint.expect("endpoint").session_bound);
}

#[test]
fn desktop_process_shutdown_observation_uses_process_snapshot() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_process_shutdown(),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "process_stopped"
    ));

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceRestarting);
    assert_eq!(
        snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 1 })
    );
    assert!(snapshot.endpoint.is_none());
    assert_eq!(
        snapshot.process_adapter.state,
        NativeProcessAdapterState::Stopped
    );
    assert!(snapshot.process_adapter.endpoint.is_none());
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Restarting
    );
}
