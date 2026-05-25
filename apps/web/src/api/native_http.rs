//! plan_ref:
//!   - 11_ui_design_02_desktop#desktop-native-adapter-contract
//!
//! Native shell HTTP URL helpers.

use super::native_bootstrap::read_native_bootstrap;

const PACKAGED_SHELL_HOST: &str = "tauri.localhost";
const LOOPBACK_DEV_HTTP_BASE: &str = "http://127.0.0.1:3001";
#[cfg(target_arch = "wasm32")]
const LOOPBACK_DEV_WS_URL: &str = "ws://127.0.0.1:3001/ws";

pub(crate) struct ApiUrl {
    pub url: String,
    pub include_credentials: bool,
}

pub(crate) fn api_url(path: &str) -> ApiUrl {
    let http_base = preferred_http_base();
    api_url_for_http_base(path, http_base.as_deref())
}

pub(crate) fn preferred_http_base() -> Option<String> {
    read_native_bootstrap()
        .http_base()
        .map(str::to_string)
        .or_else(packaged_shell_loopback_http_base)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn packaged_shell_loopback_ws_url() -> Option<String> {
    packaged_shell_loopback_http_base().map(|_| LOOPBACK_DEV_WS_URL.to_string())
}

#[cfg(target_arch = "wasm32")]
fn packaged_shell_loopback_http_base() -> Option<String> {
    let window = web_sys::window()?;
    let hostname = window.location().hostname().ok()?;
    packaged_shell_loopback_http_base_for_hostname(&hostname).map(str::to_string)
}

#[cfg(not(target_arch = "wasm32"))]
fn packaged_shell_loopback_http_base() -> Option<String> {
    None
}

fn packaged_shell_loopback_http_base_for_hostname(hostname: &str) -> Option<&'static str> {
    (hostname == PACKAGED_SHELL_HOST).then_some(LOOPBACK_DEV_HTTP_BASE)
}

fn api_url_for_http_base(path: &str, http_base: Option<&str>) -> ApiUrl {
    match http_base {
        Some(base) => ApiUrl {
            url: format!(
                "{}/{}",
                base.trim_end_matches('/'),
                path.trim_start_matches('/')
            ),
            include_credentials: true,
        },
        None => ApiUrl {
            url: path.to_string(),
            include_credentials: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{api_url_for_http_base, packaged_shell_loopback_http_base_for_hostname};

    #[test]
    fn api_url_uses_native_http_base_when_available() {
        let url = api_url_for_http_base(
            "/api/ai/backend-capabilities",
            Some("http://127.0.0.1:3001/"),
        );

        assert_eq!(url.url, "http://127.0.0.1:3001/api/ai/backend-capabilities");
        assert!(url.include_credentials);
    }

    #[test]
    fn api_url_keeps_relative_path_without_native_bootstrap() {
        let url = api_url_for_http_base("/api/repo/graph", None);

        assert_eq!(url.url, "/api/repo/graph");
        assert!(!url.include_credentials);
    }

    #[test]
    fn packaged_shell_loopback_http_base_is_limited_to_tauri_localhost() {
        assert_eq!(
            packaged_shell_loopback_http_base_for_hostname("tauri.localhost"),
            Some("http://127.0.0.1:3001")
        );
        assert_eq!(
            packaged_shell_loopback_http_base_for_hostname("127.0.0.1"),
            None
        );
        assert_eq!(
            packaged_shell_loopback_http_base_for_hostname("example.test"),
            None
        );
    }
}
