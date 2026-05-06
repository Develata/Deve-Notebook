use super::support::endpoint;
use crate::{
    MobileBootstrap, MobileServiceState, MobileSessionMaterial, MobileShell, MobileShellError,
};
use deve_core::native_adapter::{NativeProcessAdapterState, NativeServiceSupervisorState};

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
