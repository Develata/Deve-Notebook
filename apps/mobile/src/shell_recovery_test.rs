//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-service-supervisor-contract

use crate::{
    MobileLifecycleEvent, MobileServiceState, MobileSessionMaterial, MobileShell, MobileShellError,
};
use deve_core::native_adapter::{
    NativeEndpointReady, NativePlatformEventKind, NativeProcessAdapterState,
    NativeRuntimeReadiness, NativeServiceFailureKind, NativeServiceOffline,
    NativeServiceRestarting, NativeServiceSupervisorState,
};

fn endpoint() -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound: false,
    }
}

fn ready_probe() -> NativeRuntimeReadiness {
    NativeRuntimeReadiness {
        endpoint_reachable: true,
        auth_status_valid: true,
        node_role_readable: true,
        repo_handshake_complete: true,
        writer_ready: true,
        scope_nonce_current: true,
    }
}

fn bound_shell() -> MobileShell {
    let mut shell = MobileShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint()).expect("bind endpoint");
    shell
        .bind_session(MobileSessionMaterial::bound())
        .expect("bind session");
    shell
}

#[test]
fn mobile_service_offline_observation_does_not_consume_supervisor_budget() {
    let mut shell = bound_shell();

    shell.mark_service_offline("first", true);
    shell.mark_service_offline("second", true);
    shell.mark_service_offline("third", true);

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ServiceRestarting);
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
fn mobile_service_offline_retryability_is_clamped_after_failure_budget() {
    let mut shell = bound_shell();

    shell.mark_supervisor_failure(NativeServiceFailureKind::HealthProbeFailed, "first");
    shell.mark_supervisor_failure(NativeServiceFailureKind::ProcessExited, "second");
    shell.mark_service_offline("third", true);

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ServiceOffline);
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
    assert_eq!(shell.snapshot().state, MobileServiceState::ServiceOffline);
    assert_eq!(shell.snapshot().offline, terminal.offline);
    assert_eq!(shell.snapshot().supervisor, terminal.supervisor);
    assert_eq!(shell.snapshot().process_adapter, terminal.process_adapter);
}

#[test]
fn mobile_probe_timeout_observation_uses_process_snapshot() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_probe_timeout(),
        Err(MobileShellError::ServiceOffline { reason }) if reason == "probe_failed"
    ));

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ServiceRestarting);
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
fn mobile_probe_timeout_requires_endpoint_rebind_before_session_handoff() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_probe_timeout(),
        Err(MobileShellError::ServiceOffline { reason }) if reason == "probe_failed"
    ));
    let before_failed_session = shell.snapshot();
    assert!(matches!(
        shell.bind_session(MobileSessionMaterial::bound()),
        Err(MobileShellError::SessionNotBound)
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
    assert_eq!(blocked.state, MobileServiceState::ServiceRestarting);
    assert_eq!(blocked.supervisor.restart_attempt, 1);
    assert!(blocked.endpoint.is_none());

    shell.bind_endpoint(endpoint()).expect("rebind endpoint");
    shell
        .bind_session(MobileSessionMaterial::bound())
        .expect("bind session after endpoint rebind");

    let recovered = shell.snapshot();
    assert_eq!(recovered.state, MobileServiceState::SessionBound);
    assert_eq!(
        recovered.supervisor.state,
        NativeServiceSupervisorState::SessionHandoffReady
    );
    assert!(recovered.endpoint.expect("endpoint").session_bound);
}

#[test]
fn mobile_process_shutdown_observation_uses_process_snapshot() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_process_shutdown(),
        Err(MobileShellError::ServiceOffline { reason }) if reason == "process_stopped"
    ));

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ServiceRestarting);
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

#[test]
fn mobile_retryable_offline_can_restart_service() {
    let mut shell = bound_shell();
    shell.mark_service_offline("probe_failed", true);

    shell.start_service();

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ServiceStarting);
    assert_eq!(snapshot.offline, None);
    assert_eq!(snapshot.restarting, None);
    assert_eq!(snapshot.suspended, None);
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Starting
    );
    assert_eq!(snapshot.supervisor.offline, None);
}

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
