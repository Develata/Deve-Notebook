use super::support::{bound_shell, ready_probe};
use crate::{DesktopServiceState, DesktopShell};
use deve_core::native_adapter::{
    NativeRuntimeReadiness, NativeServiceFailureKind, NativeServiceOffline,
    NativeServiceRestarting, NativeServiceSupervisorState,
};

#[test]
fn desktop_supervisor_classifies_retryable_service_failures() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));
    shell.mark_supervisor_failure(NativeServiceFailureKind::BindFailed, "port_busy");

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceRestarting);
    assert_eq!(snapshot.readiness, NativeRuntimeReadiness::default());
    assert_eq!(
        snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 1 })
    );
    assert!(snapshot.endpoint.is_none());
    assert!(snapshot.process_adapter.endpoint.is_none());
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Restarting
    );
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "port_busy".to_string(),
            retryable: true,
        })
    );
    assert_eq!(
        shell
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "service_offline"
    );
}

#[test]
fn desktop_supervisor_keeps_session_handoff_failure_fatal() {
    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.mark_supervisor_failure(NativeServiceFailureKind::SessionHandoffFailed, "missing");
    let terminal = shell.snapshot();
    shell.start_service();
    assert_eq!(shell.snapshot().state, DesktopServiceState::ServiceOffline);
    assert_eq!(shell.snapshot().offline, terminal.offline);
    assert_eq!(shell.snapshot().supervisor, terminal.supervisor);
    assert_eq!(shell.snapshot().process_adapter, terminal.process_adapter);
    shell.mark_service_offline("service_dead", true);
    shell.mark_supervisor_failure(NativeServiceFailureKind::ProcessExited, "process_exited");

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceOffline);
    assert_eq!(snapshot.restarting, None);
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Offline
    );
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "missing".to_string(),
            retryable: false,
        })
    );
}
