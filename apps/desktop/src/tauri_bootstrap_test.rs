use deve_core::native_adapter::{
    NativeProcessAdapterDecision, NativeProcessAdapterPolicy, NativeProcessRuntimeState,
};

use crate::{
    DesktopBootstrap, DesktopCommandProcessLauncher, DesktopLocalServiceRuntime,
    DesktopLocalServiceTauriState, DesktopNativeSessionCookie, DesktopTauriBootstrapError,
    DesktopTauriBootstrapScript, desktop_tauri_session_invalid_init_script,
    desktop_tauri_success_init_script,
};

fn success_bootstrap() -> DesktopBootstrap {
    DesktopBootstrap {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound: true,
    }
}

fn native_session_cookie() -> DesktopNativeSessionCookie {
    DesktopNativeSessionCookie::from_set_cookie(
        "token=abc.def; Path=/; HttpOnly; SameSite=None; Secure",
        "127.0.0.1",
    )
    .expect("cookie")
}

#[test]
fn tauri_success_init_script_is_raw_js_and_session_bound() {
    let script =
        desktop_tauri_success_init_script(&success_bootstrap(), Some(native_session_cookie()))
            .expect("script");

    assert!(
        script
            .source()
            .starts_with("window.__DEVE_NATIVE_BOOTSTRAP=")
    );
    assert!(script.source().contains("\"session_bound\":true"));
    assert!(!script.source().contains("<script"));
    assert!(!script.source().contains("token"));
    assert!(!script.source().contains("secret"));
    assert!(!script.is_recovery());
    assert!(script.session_bound());
    assert!(!script.opens_authority_write_path());
    assert!(script.has_native_session_cookie());
}

#[test]
fn tauri_success_init_script_rejects_unbound_session() {
    let mut bootstrap = success_bootstrap();
    bootstrap.session_bound = false;

    assert!(matches!(
        desktop_tauri_success_init_script(&bootstrap, None),
        Err(DesktopTauriBootstrapError::SessionNotBound)
    ));
}

#[test]
fn tauri_success_init_script_requires_native_session_cookie() {
    assert!(matches!(
        desktop_tauri_success_init_script(&success_bootstrap(), None),
        Err(DesktopTauriBootstrapError::NativeSessionCookieRequired)
    ));
}

#[test]
fn tauri_success_init_script_can_carry_http_only_cookie_outside_js_source() {
    let script =
        desktop_tauri_success_init_script(&success_bootstrap(), Some(native_session_cookie()))
            .expect("script");

    assert!(script.has_native_session_cookie());
    assert!(!script.source().contains("abc.def"));
    assert!(!script.source().contains("token"));
}

#[test]
fn tauri_recovery_init_script_exposes_only_recovery_state() {
    let script = desktop_tauri_session_invalid_init_script().expect("script");

    assert!(script.is_recovery());
    assert!(!script.session_bound());
    assert!(
        script
            .source()
            .contains("\"service_state\":\"session_invalid\"")
    );
    assert!(!script.source().contains("http_base"));
    assert!(!script.source().contains("ws_base"));
    assert!(!script.source().contains("token"));
    assert!(!script.source().contains("secret"));
    assert!(!script.opens_authority_write_path());
}

#[test]
fn tauri_bootstrap_source_rejects_secret_bearing_material() {
    let result = DesktopTauriBootstrapScript::new(
        "window.__DEVE_NATIVE_BOOTSTRAP={token:\"x\"};".to_string(),
        false,
        true,
        None,
    );

    assert!(matches!(
        result,
        Err(DesktopTauriBootstrapError::ForbiddenMaterial { marker: "token" })
    ));
}

#[test]
fn tauri_state_keeps_successful_runtime_observable() {
    let runtime = DesktopLocalServiceRuntime::with_launcher(
        NativeProcessAdapterPolicy {
            decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
            child_process_runtime_enabled: true,
            embedded_service_runtime_enabled: false,
            packaging_gate_required: true,
            authority_writes_allowed: false,
        },
        1,
        DesktopCommandProcessLauncher::default(),
    );
    let state = DesktopLocalServiceTauriState::new(runtime);
    let snapshot = state.runtime_snapshot().expect("snapshot");

    assert_eq!(snapshot.state, NativeProcessRuntimeState::Disabled);
    assert!(snapshot.child_process_runtime_enabled);
    assert!(!snapshot.authority_writes_allowed);
}
