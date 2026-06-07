use super::super::{
    NativeProcessAdapter, NativeProcessAdapterSnapshot, NativeProcessAdapterState,
    NativeServiceOffline, NativeServiceSupervisor, NativeServiceSupervisorObservation,
    NativeServiceSupervisorState,
};
use super::support::{service_endpoint, service_probe};

#[test]
fn process_snapshot_drives_health_and_session_handoff() {
    let mut supervisor = NativeServiceSupervisor::new(2);
    supervisor.start();
    let mut process = NativeProcessAdapter::default();

    let endpoint_snapshot = process
        .bind_existing_endpoint(service_endpoint())
        .expect("endpoint");
    assert_eq!(
        supervisor.record_process_snapshot(&endpoint_snapshot),
        NativeServiceSupervisorObservation::EndpointHealthy
    );
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::EndpointHealthy
    );

    let session_snapshot = process.bind_session(true).expect("session");
    assert_eq!(
        supervisor.record_process_snapshot(&session_snapshot),
        NativeServiceSupervisorObservation::SessionHandoffReady
    );
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::SessionHandoffReady
    );
}

#[test]
fn process_probe_timeout_snapshot_consumes_retry_budget() {
    let mut supervisor = NativeServiceSupervisor::new(1);
    supervisor.start();
    let mut process = NativeProcessAdapter::default();
    process
        .bind_existing_endpoint(service_endpoint())
        .expect("endpoint");

    let first_timeout = process.record_probe_timeout();
    assert_eq!(
        supervisor.record_process_snapshot(&first_timeout),
        NativeServiceSupervisorObservation::Offline(NativeServiceOffline {
            reason: "probe_failed".to_string(),
            retryable: true,
        })
    );
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::Restarting
    );
    assert_eq!(supervisor.snapshot().restart_attempt, 1);

    let second_timeout = process.record_probe_timeout();
    assert_eq!(
        supervisor.record_process_snapshot(&second_timeout),
        NativeServiceSupervisorObservation::Offline(NativeServiceOffline {
            reason: "probe_failed".to_string(),
            retryable: false,
        })
    );
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::Offline
    );
}

#[test]
fn process_shutdown_snapshot_enters_restart_path() {
    let mut supervisor = NativeServiceSupervisor::new(2);
    supervisor.start();
    let mut process = NativeProcessAdapter::default();
    process
        .bind_existing_endpoint(service_endpoint())
        .expect("endpoint");

    let stopped = process.record_process_stopped();

    assert_eq!(
        supervisor.record_process_snapshot(&stopped),
        NativeServiceSupervisorObservation::Offline(NativeServiceOffline {
            reason: "process_stopped".to_string(),
            retryable: true,
        })
    );
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::Restarting
    );
}

#[test]
fn malformed_session_snapshot_is_fatal_offline() {
    let mut supervisor = NativeServiceSupervisor::new(2);
    supervisor.start();
    let snapshot = NativeProcessAdapterSnapshot {
        state: NativeProcessAdapterState::SessionHandoffReady,
        endpoint: Some(service_endpoint()),
        health_probe: service_probe(),
        child_process_runtime_enabled: false,
        embedded_service_runtime_enabled: false,
        child_process_running: false,
        authority_writes_allowed: false,
    };

    assert_eq!(
        supervisor.record_process_snapshot(&snapshot),
        NativeServiceSupervisorObservation::Offline(NativeServiceOffline {
            reason: "session_not_bound".to_string(),
            retryable: false,
        })
    );
    assert_eq!(
        supervisor.snapshot().state,
        NativeServiceSupervisorState::Offline
    );
}
