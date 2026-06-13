//! plan_ref:
//!   - 08_auth#auth-http-endpoints
//!   - 08_auth#unauthorized-handling
//!   - 08_auth#session-probe-policy
//!

use deve_core::protocol::auth::AuthStatusResponse;
use gloo_net::http::Request;
use serde_json::Value;
use web_sys::RequestCredentials;

use super::native_http::{api_url, api_url_for_http_base};

#[derive(Debug, PartialEq, Eq)]
pub enum AuthProbe {
    Valid,
    Invalid,
    Unknown,
}

pub async fn probe_auth_status() -> AuthProbe {
    probe_auth_status_with_api_url(api_url("/api/auth/status")).await
}

pub async fn probe_auth_status_with_http_base(http_base: Option<&str>) -> AuthProbe {
    probe_auth_status_with_api_url(api_url_for_http_base("/api/auth/status", http_base)).await
}

async fn probe_auth_status_with_api_url(api: super::native_http::ApiUrl) -> AuthProbe {
    let mut request = Request::get(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }

    match request.send().await {
        Ok(response) if response.ok() => match response.json::<AuthStatusResponse>().await {
            Ok(status) if status.authenticated => AuthProbe::Valid,
            Ok(_) => AuthProbe::Invalid,
            Err(_) => AuthProbe::Unknown,
        },
        Ok(response) => {
            let status = response.status();
            let has_auth_error_code = response
                .json::<Value>()
                .await
                .ok()
                .is_some_and(|payload| has_auth_error_code(&payload));
            classify_auth_probe_failure(status, has_auth_error_code)
        }
        Err(_) => AuthProbe::Unknown,
    }
}

fn classify_auth_probe_failure(status: u16, has_auth_error_code: bool) -> AuthProbe {
    if matches!(status, 401 | 403) || has_auth_error_code {
        AuthProbe::Invalid
    } else {
        AuthProbe::Unknown
    }
}

fn has_auth_error_code(payload: &Value) -> bool {
    payload
        .get("code")
        .and_then(Value::as_str)
        .is_some_and(|code| code.starts_with("AUTH_"))
}

#[cfg(test)]
mod tests {
    use super::{
        AuthProbe, api_url_for_http_base, classify_auth_probe_failure, has_auth_error_code,
    };
    use serde_json::json;

    #[test]
    fn classifies_auth_status_codes_as_invalid() {
        assert_eq!(classify_auth_probe_failure(401, false), AuthProbe::Invalid);
        assert_eq!(classify_auth_probe_failure(403, false), AuthProbe::Invalid);
    }

    #[test]
    fn classifies_auth_error_payloads_as_invalid() {
        assert!(has_auth_error_code(
            &json!({ "code": "AUTH_TOKEN_EXPIRED" })
        ));
        assert_eq!(classify_auth_probe_failure(500, true), AuthProbe::Invalid);
    }

    #[test]
    fn keeps_non_auth_failures_unknown() {
        assert!(!has_auth_error_code(&json!({ "code": "REQUEST_FAILED" })));
        assert_eq!(classify_auth_probe_failure(500, false), AuthProbe::Unknown);
    }

    #[test]
    fn auth_status_url_uses_shared_native_http_policy() {
        let api = api_url_for_http_base("/api/auth/status", Some("http://127.0.0.1:3001/"));
        assert_eq!(api.url, "http://127.0.0.1:3001/api/auth/status");
        assert!(api.include_credentials);

        let api = api_url_for_http_base("/api/auth/status", None);
        assert_eq!(api.url, "/api/auth/status");
        assert!(!api.include_credentials);
    }
}
