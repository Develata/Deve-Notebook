use super::super::{
    NativeServiceFailureKind, NativeServiceOffline, NativeServiceSupervisor,
    NativeServiceSupervisorState,
};

#[test]
fn classifies_retryable_failures_until_budget_is_exhausted() {
    let mut supervisor = NativeServiceSupervisor::new(1);
    supervisor.start();

    let first = supervisor.record_failure(NativeServiceFailureKind::BindFailed, "port_busy");
    assert!(first.retryable);
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::Restarting
    );

    let second = supervisor.record_failure(NativeServiceFailureKind::HealthProbeFailed, "dead");
    assert!(!second.retryable);
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::Offline
    );
}

#[test]
fn keeps_session_handoff_failure_fatal() {
    let mut supervisor = NativeServiceSupervisor::new(3);
    supervisor.start();

    let offline = supervisor.record_failure(
        NativeServiceFailureKind::SessionHandoffFailed,
        "session_missing",
    );

    assert!(!offline.retryable);
    let retryable_failure =
        supervisor.record_failure(NativeServiceFailureKind::ProcessExited, "process_exited");
    assert!(!retryable_failure.retryable);
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::Offline
    );
    assert_eq!(
        supervisor.snapshot().offline,
        Some(NativeServiceOffline {
            reason: "session_missing".to_string(),
            retryable: false,
        })
    );
}
