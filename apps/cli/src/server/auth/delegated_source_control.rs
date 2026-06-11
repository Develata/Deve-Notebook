//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 08_auth#jwt-cookie-contract
//!
//! Explicit capability for remote-proxy Source Control delegated writes.

use axum::http::HeaderMap;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(crate) const DELEGATED_SC_HEADER: &str = "x-deve-source-control-delegation";

const DELEGATED_SC_VERSION: &str = "v1";
const DELEGATED_SC_TRANSCRIPT: &str = "deve-source-control-delegation:v1";

pub(crate) fn header_value(signing_secret: &str) -> String {
    format!(
        "{DELEGATED_SC_VERSION}.{}",
        super::signing::hmac_sha256_hex(
            signing_secret.as_bytes(),
            DELEGATED_SC_TRANSCRIPT.as_bytes()
        )
    )
}

pub(crate) fn validate_headers(
    headers: &HeaderMap,
    signing_secret: &str,
) -> Result<(), ServerError> {
    let Some(value) = headers
        .get(DELEGATED_SC_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return Err(capability_denied(
            "delegated source control capability missing",
        ));
    };
    validate_header_value(value, signing_secret)
}

fn validate_header_value(value: &str, signing_secret: &str) -> Result<(), ServerError> {
    let Some((version, signature)) = value.split_once('.') else {
        return Err(capability_denied(
            "delegated source control capability invalid",
        ));
    };
    if version != DELEGATED_SC_VERSION || !super::signing::is_hex_digest(signature) {
        return Err(capability_denied(
            "delegated source control capability invalid",
        ));
    }
    let expected = header_value(signing_secret);
    if super::signing::constant_time_eq(value.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(capability_denied(
            "delegated source control capability invalid",
        ))
    }
}

fn capability_denied(detail: impl Into<String>) -> ServerError {
    ServerError::with_detail(ServerErrorCode::PluginCapabilityDenied, detail)
}

#[cfg(test)]
mod tests {
    use super::{DELEGATED_SC_HEADER, header_value, validate_headers};
    use axum::http::{HeaderMap, HeaderValue};
    use deve_core::protocol::ServerErrorCode;

    const SECRET: &str = "test_secret_key_at_least_32_bytes_long!";

    #[test]
    fn delegated_source_control_header_validates_with_matching_secret() {
        let mut headers = HeaderMap::new();
        headers.insert(
            DELEGATED_SC_HEADER,
            HeaderValue::from_str(&header_value(SECRET)).expect("header"),
        );

        assert!(validate_headers(&headers, SECRET).is_ok());
    }

    #[test]
    fn delegated_source_control_header_fails_closed() {
        let err = validate_headers(&HeaderMap::new(), SECRET).unwrap_err();
        assert_eq!(err.code, ServerErrorCode::PluginCapabilityDenied);

        let mut headers = HeaderMap::new();
        headers.insert(
            DELEGATED_SC_HEADER,
            HeaderValue::from_str(&header_value(SECRET)).expect("header"),
        );
        let err = validate_headers(&headers, "rotated_test_secret_at_least_32_bytes!").unwrap_err();
        assert_eq!(err.code, ServerErrorCode::PluginCapabilityDenied);
    }
}
