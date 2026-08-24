//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!
//! Native WebView bootstrap source projection tests.

use super::*;

#[test]
fn mobile_embedded_backend_android_initial_session_source_binds_markers_to_process_identity() {
    let bootstrap = MobileBootstrap {
        http_base: "http://127.0.0.1:40123".to_string(),
        ws_base: "ws://127.0.0.1:40123".to_string(),
        node_role: "main".to_string(),
        session_bound: true,
        platform_lifecycle_authority: "native",
        capabilities: deve_core::native_adapter::NativeShellCapabilities::local_backend(),
    };
    let cookie = MobileNativeSessionCookie::from_set_cookie(
        "token=cookie-value; Path=/; HttpOnly; Secure; SameSite=None",
        "127.0.0.1",
    )
    .expect("cookie");

    let first_payload = serde_json::to_string(&bootstrap).expect("first bootstrap payload");
    let replacement_payload = first_payload.clone();
    let first =
        mobile_embedded_backend_script(bootstrap.clone(), cookie.clone(), "process-session-a")
            .expect("first process script");
    let replacement = mobile_embedded_backend_script(bootstrap, cookie, "process-session-b")
        .expect("replacement process script");

    for script in [&first, &replacement] {
        assert!(
            script
                .source()
                .contains("root.__DEVE_NATIVE_SESSION_INSTALL_ID = installId")
        );
        assert!(!script.source().contains("===current.http_base"));
        assert!(!script.source().contains("cookie-value"));
    }
    let first_android = android_initial_session_prepare_source(
        WEBVIEW_BOOTSTRAP_INIT_SOURCE,
        &first_payload,
        "process-session-a",
    )
    .expect("first Android prepare source");
    let replacement_android = android_initial_session_prepare_source(
        WEBVIEW_BOOTSTRAP_INIT_SOURCE,
        &replacement_payload,
        "process-session-b",
    )
    .expect("replacement Android prepare source");
    assert!(first_android.contains("root.sessionStorage.getItem(key) === installId"));
    assert!(replacement_android.contains("root.sessionStorage.getItem(key) === installId"));
    assert!(first_android.contains("root.__DEVE_NATIVE_SESSION_STORAGE_READY === true"));
    assert!(first_android.contains("initializeBootstrap"));
    assert!(first_android.contains("initialBootstrapStatus"));
    assert!(first_android.contains("fallback.capabilities"));
    assert!(first_android.contains(",false,true)"));
    validate_mobile_embedded_script_source(&first_android)
        .expect("Android initial session source hygiene");
    assert!(first_android.contains("root.sessionStorage.getItem(key) !== installId"));
    assert!(
        first
            .source()
            .contains("Object.keys(bootstrap).sort().join")
    );
    assert!(first.source().contains("bootstrap.session_bound === true"));
    assert!(
        first
            .source()
            .contains("bootstrap.platform_lifecycle_authority === \"native\"")
    );
    assert!(
        first
            .source()
            .contains("bootstrap.node_role === fallback.node_role")
    );
    assert!(
        first
            .source()
            .contains("Object.keys(bootstrap.capabilities).join")
    );
    assert!(!first_android.contains("===current.http_base"));
    assert!(!replacement_android.contains("===current.http_base"));
    assert!(first.source().contains("process-session-a"));
    assert!(replacement.source().contains("process-session-b"));
    assert_ne!(first.source(), replacement.source());
    assert!(first.replacement_source().contains(",true);"));
}
