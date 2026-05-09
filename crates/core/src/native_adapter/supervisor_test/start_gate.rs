use super::super::{
    NativeServiceFailureKind, NativeServiceOffline, NativeServiceSupervisor,
    NativeServiceSupervisorState,
};

#[test]
fn start_does_not_clear_terminal_offline() {
    let mut supervisor = NativeServiceSupervisor::new(3);
    let fatal = supervisor.record_failure(
        NativeServiceFailureKind::SessionHandoffFailed,
        "session_missing",
    );
    assert!(!fatal.retryable);

    supervisor.start();

    let snapshot = supervisor.snapshot();
    assert_eq!(snapshot.state, NativeServiceSupervisorState::Offline);
    assert_eq!(snapshot.restart_attempt, 0);
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "session_missing".to_string(),
            retryable: false,
        })
    );
}

#[test]
fn start_does_not_clear_budget_exhausted_offline() {
    let mut supervisor = NativeServiceSupervisor::new(1);
    let retryable = supervisor.record_failure(NativeServiceFailureKind::BindFailed, "first");
    assert!(retryable.retryable);
    let terminal = supervisor.record_failure(NativeServiceFailureKind::ProcessExited, "second");
    assert!(!terminal.retryable);

    supervisor.start();

    let snapshot = supervisor.snapshot();
    assert_eq!(snapshot.state, NativeServiceSupervisorState::Offline);
    assert_eq!(snapshot.restart_attempt, 1);
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "second".to_string(),
            retryable: false,
        })
    );
}

#[test]
fn start_clears_retryable_offline_without_resetting_budget() {
    let mut supervisor = NativeServiceSupervisor::new(3);
    let retryable = supervisor.record_failure(NativeServiceFailureKind::BindFailed, "port_busy");
    assert!(retryable.retryable);

    supervisor.start();

    let snapshot = supervisor.snapshot();
    assert_eq!(snapshot.state, NativeServiceSupervisorState::Starting);
    assert_eq!(snapshot.restart_attempt, 1);
    assert_eq!(snapshot.offline, None);
}
