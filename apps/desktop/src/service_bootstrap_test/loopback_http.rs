use std::time::Duration;

use deve_core::native_adapter::NATIVE_SESSION_BOOTSTRAP_SECRET_ENV;
use serde_json::json;

use crate::{
    DesktopLocalServiceBootstrapError, DesktopLocalServiceProbe, DesktopLocalServiceSessionHandoff,
    DesktopLoopbackHttpProbe,
};

use super::support::{
    endpoint, plan, spawn_delayed_json_response, spawn_json_response,
    spawn_native_session_then_auth_status,
};

#[test]
fn desktop_loopback_http_probe_reads_node_role() {
    let node_role_base = spawn_json_response(json!({
        "role": "native-main",
        "native_service": {
            "state": "session_pending",
            "endpoint": {
                "http_base": "http://127.0.0.1:39101",
                "ws_base": "ws://127.0.0.1:39101",
                "node_role": "native-main",
                "session_bound": false
            }
        }
    }));
    let mut plan = plan();
    plan.http_base = node_role_base;
    let mut probe = DesktopLoopbackHttpProbe::default();

    let outcome = probe.probe_node_role(&plan).expect("node role probe");
    assert!(outcome.probe.is_healthy());
    assert_eq!(outcome.endpoint.node_role, "native-main");
}

#[test]
fn desktop_loopback_http_probe_retries_during_service_startup() {
    let node_role_base = spawn_delayed_json_response(
        Duration::from_millis(80),
        json!({
            "role": "native-main",
            "native_service": {
                "state": "session_pending",
                "endpoint": {
                    "http_base": "http://127.0.0.1:39101",
                    "ws_base": "ws://127.0.0.1:39101",
                    "node_role": "native-main",
                    "session_bound": false
                }
            }
        }),
    );
    let mut plan = plan();
    plan.http_base = node_role_base;
    let mut probe = DesktopLoopbackHttpProbe::new(Duration::from_millis(50), 64 * 1024)
        .with_startup_retry(Duration::from_millis(800), Duration::from_millis(20));

    let outcome = probe.probe_node_role(&plan).expect("node role probe");

    assert!(outcome.probe.is_healthy());
    assert_eq!(outcome.endpoint.node_role, "native-main");
}

#[test]
fn desktop_loopback_http_probe_requires_native_session_secret() {
    let mut plan = plan();
    plan.spawn_spec
        .env
        .retain(|binding| binding.key != NATIVE_SESSION_BOOTSTRAP_SECRET_ENV);
    let endpoint = endpoint(false);
    let mut probe = DesktopLoopbackHttpProbe::default();

    let error = probe
        .bind_session(&plan, &endpoint)
        .expect_err("missing native session secret fails closed");

    assert!(matches!(
        error,
        DesktopLocalServiceBootstrapError::MissingNativeSessionBootstrapSecret
    ));
}

#[test]
fn desktop_loopback_http_probe_issues_native_session_cookie_before_auth_status() {
    let mut plan = plan();
    plan.http_base = spawn_native_session_then_auth_status();
    let endpoint = endpoint(false);
    let mut probe = DesktopLoopbackHttpProbe::default();

    let session = probe
        .bind_session(&plan, &endpoint)
        .expect("native session");
    let cookie = session.native_session_cookie().expect("native cookie");

    assert_eq!(cookie.name(), "token");
    assert_eq!(cookie.domain(), "127.0.0.1");
    assert_eq!(cookie.path(), "/");
    assert!(cookie.http_only());
    assert!(cookie.secure());
    assert_eq!(cookie.same_site(), "None");
    assert!(!format!("{:?}", cookie).contains("native.jwt"));
}

#[test]
fn desktop_native_session_cookie_rejects_non_loopback_domain() {
    let error = crate::DesktopNativeSessionCookie::from_set_cookie(
        "token=native.jwt; Path=/; HttpOnly; SameSite=None; Secure",
        "example.com",
    )
    .expect_err("non-loopback domain rejected");

    assert!(matches!(
        error,
        crate::DesktopShellError::NativeSessionCookieInvalid
    ));
}

#[test]
fn desktop_native_session_cookie_rejects_cookie_that_cannot_cross_tauri_origin() {
    for set_cookie in [
        "token=native.jwt; Path=/; HttpOnly; SameSite=Strict; Secure",
        "token=native.jwt; Path=/; HttpOnly; SameSite=None",
    ] {
        let error = crate::DesktopNativeSessionCookie::from_set_cookie(set_cookie, "127.0.0.1")
            .expect_err("native cookie must be cross-site capable for tauri.localhost");

        assert!(matches!(
            error,
            crate::DesktopShellError::NativeSessionCookieInvalid
        ));
    }
}
