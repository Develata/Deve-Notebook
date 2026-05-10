use super::support::bound_shell;
use crate::DesktopServiceState;
use deve_core::native_adapter::{
    NativeServiceFailureKind, NativeServiceOffline, NativeServiceRestarting,
    NativeServiceSupervisorState,
};

#[test]
fn desktop_service_offline_observation_does_not_consume_supervisor_budget() {
    let mut shell = bound_shell();

    shell.mark_service_offline("first", true);
    shell.mark_service_offline("second", true);
    shell.mark_service_offline("third", true);

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceRestarting);
    assert_eq!(
        snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 0 })
    );
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Restarting
    );
    assert_eq!(snapshot.supervisor.restart_attempt, 0);
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "third".to_string(),
            retryable: true,
        })
    );
    assert_eq!(snapshot.supervisor.offline, snapshot.offline);
}

#[test]
fn desktop_service_offline_retryability_is_clamped_after_failure_budget() {
    let mut shell = bound_shell();

    shell.mark_supervisor_failure(NativeServiceFailureKind::BindFailed, "first");
    shell.mark_supervisor_failure(NativeServiceFailureKind::ProcessExited, "second");
    shell.mark_service_offline("third", true);

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceOffline);
    assert_eq!(snapshot.restarting, None);
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Offline
    );
    assert_eq!(snapshot.supervisor.restart_attempt, 2);
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "third".to_string(),
            retryable: false,
        })
    );
    assert_eq!(snapshot.supervisor.offline, snapshot.offline);

    let terminal = shell.snapshot();
    shell.start_service();
    assert_eq!(shell.snapshot().state, DesktopServiceState::ServiceOffline);
    assert_eq!(shell.snapshot().offline, terminal.offline);
    assert_eq!(shell.snapshot().supervisor, terminal.supervisor);
    assert_eq!(shell.snapshot().process_adapter, terminal.process_adapter);
}

#[test]
fn desktop_retryable_offline_can_restart_service() {
    let mut shell = bound_shell();
    shell.mark_service_offline("probe_failed", true);

    shell.start_service();

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceStarting);
    assert_eq!(snapshot.offline, None);
    assert_eq!(snapshot.restarting, None);
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Starting
    );
    assert_eq!(snapshot.supervisor.offline, None);
}
