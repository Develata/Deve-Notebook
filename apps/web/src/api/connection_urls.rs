//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 11_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design_03_mobile#mobile-native-adapter-contract
//!

use super::connection::DEV_WS_PORT;
use super::native_bootstrap::NativeBootstrapState;

pub(super) fn build_same_origin_ws_url() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return format!("ws://localhost:{DEV_WS_PORT}/ws"),
    };
    let location = window.location();
    let host = location
        .host()
        .unwrap_or_else(|_| "localhost:3001".to_string());
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    format!("{}://{}/ws", ws_scheme, host)
}

pub(super) fn build_ws_urls_for_native_state(native: &NativeBootstrapState) -> Vec<String> {
    match native {
        NativeBootstrapState::Ready(bootstrap) => return vec![bootstrap.ws_url.clone()],
        NativeBootstrapState::Blocked(_) => return Vec::new(),
        NativeBootstrapState::Absent => {}
    }

    build_inferred_ws_urls()
}

#[cfg(not(target_arch = "wasm32"))]
fn build_inferred_ws_urls() -> Vec<String> {
    vec![format!("ws://localhost:{DEV_WS_PORT}/ws")]
}

#[cfg(target_arch = "wasm32")]
fn build_inferred_ws_urls() -> Vec<String> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return vec![format!("ws://localhost:{DEV_WS_PORT}/ws")],
    };
    let location = window.location();
    let host = location
        .host()
        .unwrap_or_else(|_| "localhost:3001".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "localhost".to_string());
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    build_inferred_ws_urls_from_parts(
        host,
        hostname,
        protocol,
        query_port(),
        super::native_http::packaged_shell_loopback_ws_url(),
        cfg!(debug_assertions),
    )
}

#[cfg(any(target_arch = "wasm32", test))]
fn build_inferred_ws_urls_from_parts(
    host: String,
    hostname: String,
    protocol: String,
    query_port: Option<u16>,
    packaged_shell_loopback_ws_url: Option<String>,
    include_debug_fallbacks: bool,
) -> Vec<String> {
    let hostname = normalize_hostname(hostname);
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    let mut urls = Vec::new();

    if let Some(port) = query_port {
        push_ws_url(
            &mut urls,
            format!("{}://{}:{}/ws", ws_scheme, hostname, port),
        );
    }

    if let Some(url) = packaged_shell_loopback_ws_url {
        push_ws_url(&mut urls, url);
    }

    push_ws_url(&mut urls, format!("{}://{}/ws", ws_scheme, host));

    if include_debug_fallbacks {
        push_ws_url(
            &mut urls,
            format!("{}://{}:{}/ws", ws_scheme, hostname, DEV_WS_PORT),
        );
        push_ws_url(
            &mut urls,
            format!("{}://localhost:{}/ws", ws_scheme, DEV_WS_PORT),
        );
        push_ws_url(
            &mut urls,
            format!("{}://127.0.0.1:{}/ws", ws_scheme, DEV_WS_PORT),
        );
    }

    urls
}

#[cfg(any(target_arch = "wasm32", test))]
fn normalize_hostname(hostname: String) -> String {
    match hostname.as_str() {
        "" | "0.0.0.0" | "::" | "[::]" => "localhost".to_string(),
        _ => hostname,
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn push_ws_url(urls: &mut Vec<String>, url: String) {
    if !urls.iter().any(|current| current == &url) {
        urls.push(url);
    }
}

#[cfg(target_arch = "wasm32")]
fn query_port() -> Option<u16> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    if search.is_empty() {
        return None;
    }
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let val = params.get("ws_port")?;
    parse_ws_port(&val)
}

#[cfg(any(target_arch = "wasm32", test))]
fn parse_ws_port(value: &str) -> Option<u16> {
    match value.parse::<u16>() {
        Ok(port) if port > 0 => Some(port),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::native_bootstrap::{NativeBootstrapBlocker, NativeWebBootstrap};

    #[test]
    fn native_ready_bootstrap_replaces_inferred_ws_candidates() {
        let urls =
            build_ws_urls_for_native_state(&NativeBootstrapState::Ready(NativeWebBootstrap {
                http_base: "http://127.0.0.1:3001".to_string(),
                ws_url: "ws://127.0.0.1:3001/ws".to_string(),
            }));

        assert_eq!(urls, vec!["ws://127.0.0.1:3001/ws"]);
    }

    #[test]
    fn blocked_native_bootstrap_does_not_fall_back_to_port_discovery() {
        let urls = build_ws_urls_for_native_state(&NativeBootstrapState::Blocked(
            NativeBootstrapBlocker::SessionNotBound,
        ));

        assert!(urls.is_empty());
    }

    #[test]
    fn absent_native_bootstrap_keeps_browser_defaults_without_window() {
        let urls = build_ws_urls_for_native_state(&NativeBootstrapState::Absent);

        assert_eq!(urls, vec![format!("ws://localhost:{DEV_WS_PORT}/ws")]);
    }

    #[test]
    fn packaged_shell_loopback_precedes_tauri_same_origin_ws() {
        let urls = build_inferred_ws_urls_from_parts(
            "tauri.localhost".to_string(),
            "tauri.localhost".to_string(),
            "http:".to_string(),
            None,
            Some("ws://127.0.0.1:3001/ws".to_string()),
            false,
        );

        assert_eq!(
            urls,
            vec![
                "ws://127.0.0.1:3001/ws".to_string(),
                "ws://tauri.localhost/ws".to_string()
            ]
        );
    }

    #[test]
    fn explicit_query_port_still_precedes_packaged_shell_loopback() {
        let urls = build_inferred_ws_urls_from_parts(
            "tauri.localhost".to_string(),
            "tauri.localhost".to_string(),
            "http:".to_string(),
            Some(4010),
            Some("ws://127.0.0.1:3001/ws".to_string()),
            false,
        );

        assert_eq!(urls[0], "ws://tauri.localhost:4010/ws");
        assert_eq!(urls[1], "ws://127.0.0.1:3001/ws");
    }

    #[test]
    fn query_ws_port_rejects_invalid_or_zero_ports() {
        assert_eq!(parse_ws_port("3001"), Some(3001));
        assert_eq!(parse_ws_port("0"), None);
        assert_eq!(parse_ws_port(""), None);
        assert_eq!(parse_ws_port("not-a-port"), None);
        assert_eq!(parse_ws_port("65536"), None);
    }
}
