//! plan_ref:
//!   - 09_auth#auth-http-endpoints
//!   - 09_auth#unauthorized-handling
//!   - 09_auth#session-probe-policy
//!

use gloo_net::http::Request;
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthProbe {
    Valid,
    Invalid,
    Unknown,
}

pub async fn probe_auth_status() -> AuthProbe {
    match Request::get("/api/auth/me").send().await {
        Ok(response) if response.ok() => AuthProbe::Valid,
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
    use super::{AuthProbe, classify_auth_probe_failure, has_auth_error_code};
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
}
