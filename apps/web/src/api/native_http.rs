//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!
//! Native shell HTTP URL helpers.

use super::native_bootstrap::read_native_bootstrap;

pub(super) struct ApiUrl {
    pub url: String,
    pub include_credentials: bool,
}

pub(super) fn api_url(path: &str) -> ApiUrl {
    let http_base = read_native_bootstrap().http_base().map(str::to_string);
    api_url_for_http_base(path, http_base.as_deref())
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
    use super::api_url_for_http_base;

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
}
