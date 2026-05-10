use super::support::{bound_shell, endpoint, ready_probe};
use crate::{MobileLifecycleEvent, MobileServiceState, MobileShell, MobileShellError};
use deve_core::native_adapter::{
    NativePlatformEventKind, NativeServiceFailureKind, NativeServiceRestarting,
};

#[test]
fn mobile_terminal_offline_rejects_endpoint_without_mutating_process_adapter() {
    let mut shell = bound_shell();
    shell.mark_supervisor_failure(
        NativeServiceFailureKind::SessionHandoffFailed,
        "session_dead",
    );
    assert!(!shell.mark_runtime_ready(ready_probe()));
    let before = shell.snapshot();

    assert!(matches!(
        shell.bind_endpoint(endpoint()),
        Err(MobileShellError::ServiceOffline { reason }) if reason == "session_dead"
    ));

    let after = shell.snapshot();
    assert_eq!(after.state, MobileServiceState::ServiceOffline);
    assert_eq!(after.process_adapter, before.process_adapter);
    assert_eq!(after.supervisor, before.supervisor);
    assert!(after.endpoint.is_none());
}

#[test]
fn mobile_service_recovery_state_survives_lifecycle_events() {
    let mut restarting = bound_shell();
    restarting.handle_lifecycle_event(MobileLifecycleEvent::Background);
    assert!(restarting.snapshot().suspended.is_some());
    restarting.mark_service_offline("service_dead", true);

    assert_eq!(
        restarting.handle_lifecycle_event(MobileLifecycleEvent::Background),
        NativePlatformEventKind::Background
    );
    assert_eq!(
        restarting.handle_lifecycle_event(MobileLifecycleEvent::Resumed),
        NativePlatformEventKind::Resumed
    );
    let restarting_snapshot = restarting.snapshot();
    assert_eq!(
        restarting_snapshot.state,
        MobileServiceState::ServiceRestarting
    );
    assert_eq!(
        restarting_snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 0 })
    );
    assert_eq!(restarting_snapshot.suspended, None);
    assert_eq!(
        restarting
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "service_offline"
    );

    let mut offline = MobileShell::new();
    offline.start_service();
    offline.mark_supervisor_failure(
        NativeServiceFailureKind::SessionHandoffFailed,
        "session_dead",
    );

    assert_eq!(
        offline.handle_lifecycle_event(MobileLifecycleEvent::Suspended),
        NativePlatformEventKind::Suspended
    );
    assert_eq!(
        offline.handle_lifecycle_event(MobileLifecycleEvent::Foreground),
        NativePlatformEventKind::Foreground
    );
    let offline_snapshot = offline.snapshot();
    assert_eq!(offline_snapshot.state, MobileServiceState::ServiceOffline);
    assert_eq!(offline_snapshot.restarting, None);
    assert_eq!(offline_snapshot.suspended, None);
    assert_eq!(
        offline
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "service_offline"
    );
}
