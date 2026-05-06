//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-service-supervisor-contract

use crate::{DesktopServiceState, DesktopSessionMaterial, DesktopShell, DesktopShellError};
use deve_core::native_adapter::{
    NativeEndpointReady, NativePlatformEventEffect, NativePlatformEventKind,
    NativeProcessAdapterState, NativeRuntimeReadiness, NativeServiceFailureKind,
    NativeServiceOffline, NativeServiceRestarting, NativeServiceSupervisorState,
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

fn bound_shell() -> DesktopShell {
    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint()).expect("bind endpoint");
    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session");
    shell
}

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
fn desktop_probe_timeout_observation_uses_process_snapshot() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_probe_timeout(),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "probe_failed"
    ));

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceRestarting);
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
fn desktop_probe_timeout_requires_endpoint_rebind_before_session_handoff() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_probe_timeout(),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "probe_failed"
    ));
    let before_failed_session = shell.snapshot();
    assert!(matches!(
        shell.bind_session(DesktopSessionMaterial::bound()),
        Err(DesktopShellError::SessionNotBound)
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
    assert_eq!(blocked.state, DesktopServiceState::ServiceRestarting);
    assert_eq!(blocked.supervisor.restart_attempt, 1);
    assert!(blocked.endpoint.is_none());

    shell.bind_endpoint(endpoint()).expect("rebind endpoint");
    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session after endpoint rebind");

    let recovered = shell.snapshot();
    assert_eq!(recovered.state, DesktopServiceState::SessionBound);
    assert_eq!(
        recovered.supervisor.state,
        NativeServiceSupervisorState::SessionHandoffReady
    );
    assert!(recovered.endpoint.expect("endpoint").session_bound);
}

#[test]
fn desktop_process_shutdown_observation_uses_process_snapshot() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    assert!(matches!(
        shell.mark_process_shutdown(),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "process_stopped"
    ));

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceRestarting);
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
