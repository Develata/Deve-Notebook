use super::super::{
    NativeServiceFailureKind, NativeServiceOffline, NativeServiceSupervisor,
    NativeServiceSupervisorObservation, NativeServiceSupervisorState,
};
use super::support::ready_process_snapshot;

#[test]
fn records_external_offline_state_without_losing_reason() {
    let mut supervisor = NativeServiceSupervisor::new(2);
    supervisor.start();
    assert_eq!(
        supervisor.record_process_snapshot(&ready_process_snapshot()),
        NativeServiceSupervisorObservation::SessionHandoffReady
    );

    supervisor.record_service_offline(NativeServiceOffline {
        reason: "service_dead".to_string(),
        retryable: true,
    });

    let snapshot = supervisor.snapshot();
    assert_eq!(snapshot.state, NativeServiceSupervisorState::Restarting);
    assert_eq!(snapshot.restart_attempt, 0);
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "service_dead".to_string(),
            retryable: true,
        })
    );
}

#[test]
fn external_offline_observation_does_not_consume_restart_budget() {
    let mut supervisor = NativeServiceSupervisor::new(1);

    let first = supervisor.record_service_offline(NativeServiceOffline {
        reason: "probe_failed".to_string(),
        retryable: true,
    });
    let second = supervisor.record_service_offline(NativeServiceOffline {
        reason: "still_dead".to_string(),
        retryable: true,
    });

    assert!(first.retryable);
    assert!(second.retryable);
    let snapshot = supervisor.snapshot();
    assert_eq!(snapshot.state, NativeServiceSupervisorState::Restarting);
    assert_eq!(snapshot.restart_attempt, 0);
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "still_dead".to_string(),
            retryable: true,
        })
    );
}

#[test]
fn external_offline_retryability_is_clamped_after_failure_budget_is_exhausted() {
    let mut supervisor = NativeServiceSupervisor::new(1);
    let failure = supervisor.record_failure(NativeServiceFailureKind::BindFailed, "port_busy");
    assert!(failure.retryable);

    let observed = supervisor.record_service_offline(NativeServiceOffline {
        reason: "still_dead".to_string(),
        retryable: true,
    });

    assert!(!observed.retryable);
    let snapshot = supervisor.snapshot();
    assert_eq!(snapshot.state, NativeServiceSupervisorState::Offline);
    assert_eq!(snapshot.restart_attempt, 1);
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "still_dead".to_string(),
            retryable: false,
        })
    );
}

#[test]
fn external_offline_observation_cannot_upgrade_terminal_offline() {
    let mut supervisor = NativeServiceSupervisor::new(3);
    let fatal = supervisor.record_failure(
        NativeServiceFailureKind::SessionHandoffFailed,
        "session_missing",
    );
    assert!(!fatal.retryable);

    let observed = supervisor.record_service_offline(NativeServiceOffline {
        reason: "service_dead".to_string(),
        retryable: true,
    });

    assert!(!observed.retryable);
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
