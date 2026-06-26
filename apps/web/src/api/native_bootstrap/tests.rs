use super::*;

fn parse(http_base: &str, ws_base: &str, session_bound: bool) -> NativeBootstrapState {
    parse_native_bootstrap_fields(
        Some(http_base.to_string()),
        Some(ws_base.to_string()),
        Some("main".to_string()),
        Some(session_bound),
        None,
    )
}

#[test]
fn accepts_session_bound_loopback_native_bootstrap() {
    assert_eq!(
        parse("http://127.0.0.1:3001/", "ws://localhost:3001/", true),
        NativeBootstrapState::Ready(NativeWebBootstrap {
            http_base: "http://127.0.0.1:3001".to_string(),
            ws_url: "ws://localhost:3001/ws".to_string(),
        })
    );
}

#[test]
fn rejects_native_bootstrap_without_session_binding() {
    assert_eq!(
        parse("http://127.0.0.1:3001", "ws://127.0.0.1:3001", false),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::SessionNotBound)
    );
}

#[test]
fn rejects_non_loopback_native_bootstrap() {
    assert_eq!(
        parse("http://192.168.1.10:3001", "ws://127.0.0.1:3001", true),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidEndpoint)
    );
}

#[test]
fn rejects_native_bootstrap_with_invalid_or_zero_port() {
    assert_eq!(
        parse("http://127.0.0.1:0", "ws://127.0.0.1:3001", true),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidEndpoint)
    );
    assert_eq!(
        parse("http://127.0.0.1:65536", "ws://127.0.0.1:3001", true),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidEndpoint)
    );
}

#[test]
fn rejects_missing_native_bootstrap_fields() {
    assert_eq!(
        parse_native_bootstrap_fields(
            Some("http://127.0.0.1:3001".to_string()),
            None,
            None,
            Some(true),
            None,
        ),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape)
    );
}

#[test]
fn rejects_native_bootstrap_without_node_role() {
    assert_eq!(
        parse_native_bootstrap_fields(
            Some("http://127.0.0.1:3001".to_string()),
            Some("ws://127.0.0.1:3001".to_string()),
            None,
            Some(true),
            None,
        ),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape)
    );
}

#[test]
fn native_shell_origin_without_bootstrap_is_invalid() {
    assert_eq!(
        absent_native_bootstrap_state(Some("tauri.localhost"), Some("http:")),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape)
    );
    assert_eq!(
        absent_native_bootstrap_state(Some("localhost"), Some("tauri:")),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape)
    );
}

#[test]
fn browser_origin_without_bootstrap_remains_absent() {
    assert_eq!(
        absent_native_bootstrap_state(Some("127.0.0.1"), Some("http:")),
        NativeBootstrapState::Absent
    );
    assert_eq!(
        absent_native_bootstrap_state(Some("example.test"), Some("https:")),
        NativeBootstrapState::Absent
    );
}

#[test]
fn maps_native_service_offline_to_blocked_state() {
    assert_eq!(
        parse_native_bootstrap_fields(None, None, None, None, Some("service_offline".into())),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::ServiceOffline)
    );
}

#[test]
fn rejects_recovery_bootstrap_with_endpoint_fields() {
    assert_eq!(
        parse_native_bootstrap_fields(
            Some("http://127.0.0.1:3001".to_string()),
            Some("ws://127.0.0.1:3001".to_string()),
            Some("main".to_string()),
            Some(true),
            Some("service_offline".into()),
        ),
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape)
    );
}

#[test]
fn maps_native_session_invalid_to_unauthorized_status() {
    assert_eq!(
        parse_native_bootstrap_fields(None, None, None, None, Some("session_invalid".into()))
            .blocked_status(),
        Some(ConnectionStatus::Unauthorized)
    );
}

#[test]
fn maps_native_foreground_reprobe_to_recovery_status() {
    assert_eq!(
        parse_native_bootstrap_fields(None, None, None, None, Some("foreground_reprobe".into()))
            .blocked_status(),
        Some(ConnectionStatus::NativeReprobeRequired)
    );
}
