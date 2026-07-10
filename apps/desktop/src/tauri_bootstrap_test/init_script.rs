use deve_core::native_adapter::{NativeAdapterError, NativeRemoteTarget};

use crate::{
    DesktopTauriBootstrapError, DesktopTauriBootstrapScript,
    desktop_tauri_remote_browser_init_script, desktop_tauri_session_invalid_init_script,
    desktop_tauri_success_init_script,
};

use super::{native_session_cookie, success_bootstrap};

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
fn tauri_remote_browser_init_script_navigates_without_native_bootstrap() {
    let script = desktop_tauri_remote_browser_init_script(&NativeRemoteTarget {
        https_origin: "https://deve.example".to_string(),
    })
    .expect("remote script");

    assert_eq!(
        script.source(),
        "(()=>{const target=new URL(\"https://deve.example\").origin;if(window.top===window&&window.location.origin!==target){window.location.replace(target);}})();"
    );
    assert!(!script.source().contains("__DEVE_NATIVE_BOOTSTRAP"));
    assert!(!script.source().contains("http_base"));
    assert!(!script.source().contains("ws_base"));
    assert!(!script.session_bound());
    assert!(!script.has_native_session_cookie());
    assert!(!script.opens_authority_write_path());
}

#[test]
fn tauri_remote_browser_init_script_does_not_replace_matching_origin() {
    let script = desktop_tauri_remote_browser_init_script(&NativeRemoteTarget {
        https_origin: "https://deve.example".to_string(),
    })
    .expect("remote script");

    let guard = script
        .source()
        .find("window.top===window&&window.location.origin!==target")
        .expect("same-origin guard");
    let replace = script
        .source()
        .find("window.location.replace(target)")
        .expect("remote navigation");
    assert!(guard < replace, "same-origin guard must precede navigation");
}

#[test]
fn tauri_remote_browser_init_script_normalizes_default_https_port() {
    let script = desktop_tauri_remote_browser_init_script(&NativeRemoteTarget {
        https_origin: "https://deve.example:443".to_string(),
    })
    .expect("remote script");

    assert!(
        script
            .source()
            .contains("new URL(\"https://deve.example:443\").origin")
    );
    assert!(
        script
            .source()
            .contains("window.top===window&&window.location.origin!==target")
    );
    assert!(script.source().contains("window.location.replace(target)"));
}

#[test]
fn tauri_remote_browser_init_script_never_navigates_subframes() {
    let script = desktop_tauri_remote_browser_init_script(&NativeRemoteTarget {
        https_origin: "https://deve.example".to_string(),
    })
    .expect("remote script");

    let main_frame_guard = script
        .source()
        .find("window.top===window")
        .expect("main-frame guard");
    let replace = script
        .source()
        .find("window.location.replace(target)")
        .expect("remote navigation");
    assert!(
        main_frame_guard < replace,
        "main-frame guard must precede navigation"
    );
}

#[test]
fn tauri_remote_browser_init_script_rejects_non_https_origin() {
    let err = desktop_tauri_remote_browser_init_script(&NativeRemoteTarget {
        https_origin: "http://deve.example".to_string(),
    })
    .expect_err("http remote target must fail");

    assert!(matches!(
        err,
        DesktopTauriBootstrapError::RemoteTarget(NativeAdapterError::WrongScheme {
            expected_scheme: "https",
            ..
        })
    ));
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
