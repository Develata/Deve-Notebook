use super::support::{bound_shell, endpoint, ready_probe};
use crate::{DesktopServiceState, DesktopShell, DesktopShellError};
use deve_core::native_adapter::{
    NativePlatformEventEffect, NativePlatformEventKind, NativeProcessAdapterState,
    NativeServiceFailureKind, NativeServiceRestarting,
};

#[test]
fn desktop_terminal_offline_rejects_endpoint_without_mutating_process_adapter() {
    let mut shell = bound_shell();
    shell.mark_supervisor_failure(NativeServiceFailureKind::SessionHandoffFailed, "missing");
    assert!(!shell.mark_runtime_ready(ready_probe()));
    let before = shell.snapshot();

    assert!(matches!(
        shell.bind_endpoint(endpoint()),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "missing"
    ));

    let after = shell.snapshot();
    assert_eq!(after.state, DesktopServiceState::ServiceOffline);
    assert_eq!(after.process_adapter, before.process_adapter);
    assert_eq!(after.supervisor, before.supervisor);
    assert!(after.endpoint.is_none());
}

#[test]
fn desktop_service_recovery_state_survives_foreground_events() {
    let mut restarting = bound_shell();
    restarting.mark_service_offline("service_dead", true);

    let effect = restarting.handle_platform_event(NativePlatformEventKind::Foreground);

    assert_eq!(effect, NativePlatformEventEffect::NoBusinessStateChange);
    let restarting_snapshot = restarting.snapshot();
    assert_eq!(
        restarting_snapshot.state,
        DesktopServiceState::ServiceRestarting
    );
    assert_eq!(
        restarting_snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 0 })
    );
    assert_eq!(
        restarting
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "service_offline"
    );

    let mut offline = DesktopShell::new();
    offline.start_service();
    offline.mark_supervisor_failure(NativeServiceFailureKind::SessionHandoffFailed, "missing");

    let effect = offline.handle_platform_event(NativePlatformEventKind::Resumed);

    assert_eq!(effect, NativePlatformEventEffect::NoBusinessStateChange);
    let offline_snapshot = offline.snapshot();
    assert_eq!(offline_snapshot.state, DesktopServiceState::ServiceOffline);
    assert_eq!(offline_snapshot.restarting, None);
    assert_eq!(
        offline
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "service_offline"
    );
}

#[test]
fn desktop_shell_session_invalid_blocks_bootstrap() {
    let mut shell = bound_shell();
    shell.invalidate_session();

    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(DesktopShellError::SessionInvalid)
    ));
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionInvalid);
    assert!(!shell.snapshot().readiness.auth_status_valid);
    assert_eq!(
        shell.snapshot().process_adapter.state,
        NativeProcessAdapterState::ExistingEndpointBound
    );
    assert_eq!(
        shell
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "session_invalid"
    );
}
