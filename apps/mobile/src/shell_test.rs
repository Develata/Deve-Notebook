//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract

use crate::{
    MobileBootstrap, MobileLifecycleEvent, MobileServiceState, MobileSessionMaterial, MobileShell,
    MobileShellError,
};
use deve_core::native_adapter::{
    CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY, CURRENT_NATIVE_PROCESS_ADAPTER_POLICY,
    NativeEndpointReady, NativePackagingDependencyGateDecision, NativePlatformEventKind,
    NativeProcessAdapterDecision, NativeProcessAdapterState, NativeRuntimeReadiness,
    NativeServiceFailureKind, NativeServiceOffline, NativeServiceRestarting,
    NativeServiceSupervisorState,
};

fn endpoint() -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "mobile-main".to_string(),
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
fn mobile_shell_injects_bootstrap_only_after_session_binding() {
    let mut shell = MobileShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint()).expect("bind endpoint");

    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(MobileShellError::InvalidEndpoint(
            deve_core::native_adapter::NativeAdapterError::SessionNotBound
        ))
    ));

    shell
        .bind_session(MobileSessionMaterial::bound())
        .expect("bind session");
    let bootstrap = shell.bootstrap_for_web().expect("bootstrap");

    assert_eq!(bootstrap.http_base, "http://127.0.0.1:3001");
    assert_eq!(bootstrap.ws_base, "ws://127.0.0.1:3001");
    assert!(bootstrap.session_bound);
    assert_eq!(shell.snapshot().state, MobileServiceState::WebShellLoading);
    assert_eq!(
        shell.snapshot().supervisor.state,
        NativeServiceSupervisorState::SessionHandoffReady
    );
    assert_eq!(
        shell.snapshot().process_adapter.state,
        NativeProcessAdapterState::SessionHandoffReady
    );
    assert!(shell.snapshot().process_adapter.is_default_safe_boundary());
}

#[test]
fn mobile_bootstrap_script_exposes_endpoint_but_not_session_secret() {
    let bootstrap = MobileBootstrap {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "mobile-main".to_string(),
        session_bound: true,
    };

    let script = bootstrap.script_tag().expect("script");

    assert!(script.contains("window.__DEVE_NATIVE_BOOTSTRAP"));
    assert!(script.contains("http://127.0.0.1:3001"));
    assert!(script.contains("\"session_bound\":true"));
    assert!(!script.contains("token"));
    assert!(!script.contains("secret"));
}

#[test]
fn mobile_shell_rejects_non_loopback_service_endpoint() {
    let mut shell = MobileShell::new();
    let mut endpoint = endpoint();
    endpoint.ws_base = "ws://192.168.1.10:3001".to_string();

    assert!(matches!(
        shell.bind_endpoint(endpoint),
        Err(MobileShellError::InvalidEndpoint(
            deve_core::native_adapter::NativeAdapterError::NonLoopbackHost { field: "ws_base" }
        ))
    ));
}

#[test]
fn mobile_background_resume_requires_fresh_reprobe_before_write() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    let background = shell.handle_lifecycle_event(MobileLifecycleEvent::Background);
    assert_eq!(background, NativePlatformEventKind::Background);
    assert_eq!(
        shell.snapshot().state,
        MobileServiceState::BackgroundSuspended
    );
    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(MobileShellError::ForegroundReprobeRequired)
    ));

    let resumed = shell.handle_lifecycle_event(MobileLifecycleEvent::Resumed);
    assert_eq!(resumed, NativePlatformEventKind::Resumed);
    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ForegroundReprobe);
    assert!(!snapshot.readiness.auth_status_valid);
    assert!(!snapshot.readiness.node_role_readable);
    assert!(!snapshot.readiness.repo_handshake_complete);
    assert!(!snapshot.readiness.writer_ready);
    assert!(!snapshot.readiness.scope_nonce_current);
    assert_eq!(
        shell
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "foreground_reprobe"
    );
}

#[test]
fn mobile_reprobe_does_not_restore_write_without_current_scope_nonce() {
    let mut shell = bound_shell();
    shell.handle_lifecycle_event(MobileLifecycleEvent::Resumed);
    let ready_without_scope = NativeRuntimeReadiness {
        scope_nonce_current: false,
        ..ready_probe()
    };

    assert!(!shell.complete_foreground_reprobe(ready_without_scope));
    assert_eq!(
        shell.snapshot().state,
        MobileServiceState::ForegroundReprobe
    );
    assert!(shell.complete_foreground_reprobe(ready_probe()));
    assert_eq!(shell.snapshot().state, MobileServiceState::RuntimeReady);
}

#[test]
fn mobile_network_events_are_hints_not_write_grants() {
    let mut shell = bound_shell();
    let event = shell.handle_lifecycle_event(MobileLifecycleEvent::NetworkOffline);

    assert_eq!(event, NativePlatformEventKind::NetworkOffline);
    assert_eq!(shell.snapshot().state, MobileServiceState::SessionBound);
    assert!(!shell.snapshot().readiness.writer_ready);
}

#[test]
fn mobile_shell_offline_and_session_invalid_block_bootstrap() {
    let mut offline = bound_shell();
    offline.mark_service_offline("service_dead", true);

    assert!(matches!(
        offline.bootstrap_for_web(),
        Err(MobileShellError::ServiceOffline { reason }) if reason == "service_dead"
    ));
    let offline_script = offline
        .recovery_bootstrap_for_web()
        .expect("recovery bootstrap")
        .script_tag()
        .expect("recovery script");
    assert!(offline_script.contains("\"service_state\":\"service_offline\""));
    assert!(!offline_script.contains("service_dead"));
    let offline_snapshot = offline.snapshot();
    assert_eq!(
        offline_snapshot.state,
        MobileServiceState::ServiceRestarting
    );
    assert_eq!(
        offline_snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 0 })
    );
    assert_eq!(
        offline_snapshot.readiness,
        NativeRuntimeReadiness::default()
    );
    assert_eq!(
        offline_snapshot.supervisor.state,
        NativeServiceSupervisorState::Restarting
    );
    assert_eq!(
        offline_snapshot.supervisor.offline,
        offline_snapshot.offline
    );
    assert!(offline_snapshot.endpoint.is_none());
    assert!(offline_snapshot.process_adapter.endpoint.is_none());

    let mut invalid = bound_shell();
    invalid.invalidate_session();
    assert!(matches!(
        invalid.bootstrap_for_web(),
        Err(MobileShellError::SessionInvalid)
    ));
    assert!(!invalid.snapshot().endpoint.expect("endpoint").session_bound);
    assert_eq!(
        invalid
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "session_invalid"
    );
    assert_eq!(
        invalid.snapshot().process_adapter.state,
        NativeProcessAdapterState::ExistingEndpointBound
    );
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

#[test]
fn mobile_default_build_defers_real_process_adapter() {
    let policy = CURRENT_NATIVE_PROCESS_ADAPTER_POLICY;

    assert_eq!(
        policy.decision,
        NativeProcessAdapterDecision::DeferredUntilPackagingGate
    );
    assert!(policy.is_deferred_no_runtime());
    assert!(!policy.child_process_runtime_enabled);
    assert!(!policy.authority_writes_allowed);
}

#[test]
fn mobile_default_build_keeps_packaging_dependency_gate_closed() {
    let policy = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

    assert_eq!(
        policy.decision,
        NativePackagingDependencyGateDecision::DeferredUntilRuntimeBatch
    );
    assert!(policy.is_deferred_no_dependency());
    assert!(!policy.real_tauri_dependencies_allowed);
    assert!(policy.default_build_remains_no_tauri);
    assert!(!policy.authority_writes_allowed);
}
