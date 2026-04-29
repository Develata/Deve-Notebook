//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract

use crate::{
    DesktopBootstrap, DesktopServiceState, DesktopSessionMaterial, DesktopShell, DesktopShellError,
};
use deve_core::native_adapter::{
    CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY, CURRENT_NATIVE_PROCESS_ADAPTER_POLICY,
    NativeEndpointReady, NativePackagingDependencyGateDecision, NativeProcessAdapterDecision,
    NativeServiceFailureKind, NativeServiceOffline, NativeServiceSupervisorState,
};

fn endpoint() -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound: false,
    }
}

#[test]
fn desktop_shell_injects_bootstrap_only_after_session_binding() {
    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint()).expect("bind endpoint");

    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(DesktopShellError::InvalidEndpoint(
            deve_core::native_adapter::NativeAdapterError::SessionNotBound
        ))
    ));

    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session");
    let bootstrap = shell.bootstrap_for_web().expect("bootstrap");

    assert_eq!(bootstrap.http_base, "http://127.0.0.1:3001");
    assert_eq!(bootstrap.ws_base, "ws://127.0.0.1:3001");
    assert!(bootstrap.session_bound);
    assert_eq!(shell.snapshot().state, DesktopServiceState::WebShellLoading);
    assert_eq!(
        shell.snapshot().supervisor.state,
        NativeServiceSupervisorState::SessionHandoffReady
    );
}

#[test]
fn desktop_bootstrap_script_exposes_endpoint_but_not_session_secret() {
    let bootstrap = DesktopBootstrap {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
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
fn desktop_shell_rejects_non_loopback_service_endpoint() {
    let mut shell = DesktopShell::new();
    let mut endpoint = endpoint();
    endpoint.http_base = "http://192.168.1.10:3001".to_string();

    assert!(matches!(
        shell.bind_endpoint(endpoint),
        Err(DesktopShellError::InvalidEndpoint(
            deve_core::native_adapter::NativeAdapterError::NonLoopbackHost { field: "http_base" }
        ))
    ));
}

#[test]
fn desktop_shell_offline_state_blocks_bootstrap_and_reports_recovery() {
    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint()).expect("bind endpoint");
    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session");
    shell.mark_service_offline("bind_failed", true);

    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "bind_failed"
    ));
    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceOffline);
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "bind_failed".to_string(),
            retryable: true,
        })
    );
    let recovery = shell
        .recovery_bootstrap_for_web()
        .expect("recovery bootstrap");
    let script = recovery.script_tag().expect("recovery script");
    assert!(script.contains("\"service_state\":\"service_offline\""));
    assert!(!script.contains("bind_failed"));
    assert!(!script.contains("secret"));
}

#[test]
fn desktop_shell_session_invalid_blocks_bootstrap() {
    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint()).expect("bind endpoint");
    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session");
    shell.invalidate_session();

    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(DesktopShellError::SessionInvalid)
    ));
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionInvalid);
    assert_eq!(
        shell
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "session_invalid"
    );
}

#[test]
fn desktop_supervisor_classifies_retryable_service_failures() {
    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.mark_supervisor_failure(NativeServiceFailureKind::BindFailed, "port_busy");

    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceOffline);
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

    let snapshot = shell.snapshot();
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

#[test]
fn desktop_default_build_defers_real_process_adapter() {
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
fn desktop_default_build_keeps_packaging_dependency_gate_closed() {
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
