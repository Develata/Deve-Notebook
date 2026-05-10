use super::support::{bound_shell, ready_probe};
use crate::{MobileServiceState, MobileShell, MobileShellError};
use deve_core::native_adapter::{
    NativeProcessAdapterState, NativeRuntimeReadiness, NativeServiceFailureKind,
    NativeServiceOffline, NativeServiceRestarting, NativeServiceSupervisorState,
};

#[test]
fn mobile_supervisor_failure_blocks_endpoint_and_reports_retryability() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));
    shell.mark_supervisor_failure(NativeServiceFailureKind::HealthProbeFailed, "probe_failed");

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ServiceRestarting);
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
            reason: "probe_failed".to_string(),
            retryable: true,
        })
    );
    assert_eq!(
        snapshot.process_adapter.state,
        NativeProcessAdapterState::Stopped
    );
    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(MobileShellError::ServiceOffline { reason }) if reason == "probe_failed"
    ));
}

#[test]
fn mobile_supervisor_session_handoff_failure_is_not_retryable() {
    let mut shell = MobileShell::new();
    shell.start_service();
    shell.mark_supervisor_failure(
        NativeServiceFailureKind::SessionHandoffFailed,
        "session_dead",
    );
    let terminal = shell.snapshot();
    shell.start_service();
    assert_eq!(shell.snapshot().state, MobileServiceState::ServiceOffline);
    assert_eq!(shell.snapshot().offline, terminal.offline);
    assert_eq!(shell.snapshot().supervisor, terminal.supervisor);
    assert_eq!(shell.snapshot().process_adapter, terminal.process_adapter);
    shell.mark_service_offline("service_dead", true);
    shell.mark_supervisor_failure(NativeServiceFailureKind::ProcessExited, "process_exited");

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ServiceOffline);
    assert_eq!(snapshot.restarting, None);
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Offline
    );
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "session_dead".to_string(),
            retryable: false,
        })
    );
}
