use super::support::{bound_shell, endpoint};
use crate::{
    DesktopBootstrap, DesktopServiceState, DesktopSessionMaterial, DesktopShell, DesktopShellError,
};
use deve_core::native_adapter::{
    NativeProcessAdapterState, NativeRuntimeReadiness, NativeServiceOffline,
    NativeServiceRestarting, NativeServiceSupervisorState,
};

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
    assert_eq!(
        shell.snapshot().process_adapter.state,
        NativeProcessAdapterState::SessionHandoffReady
    );
    assert!(shell.snapshot().process_adapter.is_default_safe_boundary());
    assert!(shell.snapshot().readiness.endpoint_reachable);
    assert!(shell.snapshot().readiness.auth_status_valid);
    assert!(shell.snapshot().readiness.node_role_readable);
}

#[test]
fn desktop_bootstrap_script_exposes_endpoint_but_not_session_secret() {
    let bootstrap = DesktopBootstrap {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound: true,
        capabilities: deve_core::native_adapter::NativeShellCapabilities::local_backend(),
    };

    let script = bootstrap.script_tag().expect("script");

    assert!(script.contains("window.__DEVE_NATIVE_BOOTSTRAP"));
    assert!(script.contains("http://127.0.0.1:3001"));
    assert!(script.contains("\"session_bound\":true"));
    assert!(script.contains("\"backend_preference_control\":true"));
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
    let mut shell = bound_shell();
    shell.mark_service_offline("bind_failed", true);

    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(DesktopShellError::ServiceOffline { reason }) if reason == "bind_failed"
    ));
    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ServiceRestarting);
    assert_eq!(snapshot.readiness, NativeRuntimeReadiness::default());
    assert_eq!(
        snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 0 })
    );
    assert_eq!(
        snapshot.offline,
        Some(NativeServiceOffline {
            reason: "bind_failed".to_string(),
            retryable: true,
        })
    );
    assert_eq!(
        snapshot.supervisor.state,
        NativeServiceSupervisorState::Restarting
    );
    assert_eq!(snapshot.supervisor.offline, snapshot.offline);
    assert!(snapshot.endpoint.is_none());
    assert!(snapshot.process_adapter.endpoint.is_none());
    assert_eq!(
        snapshot.process_adapter.state,
        NativeProcessAdapterState::Stopped
    );
    let recovery = shell
        .recovery_bootstrap_for_web()
        .expect("recovery bootstrap");
    let script = recovery.script_tag().expect("recovery script");
    assert!(script.contains("\"service_state\":\"service_offline\""));
    assert!(!script.contains("bind_failed"));
    assert!(!script.contains("secret"));
}
