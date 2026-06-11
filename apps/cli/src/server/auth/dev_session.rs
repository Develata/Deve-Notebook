//! plan_ref:
//!   - 08_auth#jwt-cookie-contract
//!   - 08_auth#localhost-dev-policy
//!
//! Anonymous localhost dev-session cookie helpers.

use axum::http::{HeaderMap, HeaderValue, header::SET_COOKIE};
use axum_extra::extract::cookie::{Cookie, SameSite};

pub(crate) const DEV_SESSION_COOKIE_NAME: &str = "deve_dev_session";
const DEV_SESSION_COOKIE_VERSION: &str = "v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DevSessionCookie {
    value: String,
    set_cookie: Option<String>,
}

impl DevSessionCookie {
    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    pub(crate) fn set_cookie(&self) -> Option<&str> {
        self.set_cookie.as_deref()
    }
}

pub(crate) fn resolve_from_cookie_header(
    cookie_header: Option<&str>,
    signing_secret: &str,
) -> DevSessionCookie {
    if let Some(value) = cookie_header
        .and_then(|header| {
            super::cookie::extract_named_cookie_from_header(header, DEV_SESSION_COOKIE_NAME)
        })
        .and_then(|value| verify_dev_session_cookie_value(&value, signing_secret))
    {
        return DevSessionCookie {
            value,
            set_cookie: None,
        };
    }

    let value = issue_dev_session_value();
    let set_cookie = Some(build_dev_session_cookie(&sign_dev_session_cookie_value(
        &value,
        signing_secret,
    )));
    DevSessionCookie { value, set_cookie }
}

pub(crate) fn append_set_cookie(headers: &mut HeaderMap, cookie: &DevSessionCookie) {
    let Some(set_cookie) = cookie.set_cookie() else {
        return;
    };
    if let Ok(value) = HeaderValue::from_str(set_cookie) {
        headers.append(SET_COOKIE, value);
    }
}

#[cfg(test)]
pub(crate) fn cookie_header_for_test(signing_secret: &str, value: &str) -> String {
    format!(
        "{DEV_SESSION_COOKIE_NAME}={}",
        sign_dev_session_cookie_value(value, signing_secret)
    )
}

fn issue_dev_session_value() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn build_dev_session_cookie(value: &str) -> String {
    Cookie::build((DEV_SESSION_COOKIE_NAME, value.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .build()
        .to_string()
}

fn sign_dev_session_cookie_value(nonce: &str, signing_secret: &str) -> String {
    format!(
        "{DEV_SESSION_COOKIE_VERSION}.{nonce}.{}",
        dev_session_signature(nonce, signing_secret)
    )
}

fn verify_dev_session_cookie_value(value: &str, signing_secret: &str) -> Option<String> {
    let mut parts = value.split('.');
    let version = parts.next()?;
    let nonce = parts.next()?;
    let signature = parts.next()?;
    if parts.next().is_some()
        || version != DEV_SESSION_COOKIE_VERSION
        || !is_valid_dev_session_nonce(nonce)
        || !is_valid_dev_session_signature(signature)
    {
        return None;
    }
    let expected = dev_session_signature(nonce, signing_secret);
    if constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
        Some(nonce.to_string())
    } else {
        None
    }
}

fn dev_session_signature(nonce: &str, signing_secret: &str) -> String {
    super::signing::hmac_sha256_hex(
        signing_secret.as_bytes(),
        format!("deve-dev-session:{DEV_SESSION_COOKIE_VERSION}:{nonce}").as_bytes(),
    )
}

fn is_valid_dev_session_nonce(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn is_valid_dev_session_signature(value: &str) -> bool {
    super::signing::is_hex_digest(value)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    super::signing::constant_time_eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::{DEV_SESSION_COOKIE_NAME, cookie_header_for_test, resolve_from_cookie_header};

    const SECRET: &str = "test_secret_key_at_least_32_bytes_long!";

    #[test]
    fn resolves_existing_dev_session_cookie_without_reissuing() {
        let session =
            resolve_from_cookie_header(Some(&cookie_header_for_test(SECRET, "abc_123")), SECRET);

        assert_eq!(session.value(), "abc_123");
        assert!(session.set_cookie().is_none());
    }

    #[test]
    fn missing_dev_session_cookie_issues_http_only_cookie() {
        let session = resolve_from_cookie_header(None, SECRET);
        let set_cookie = session.set_cookie().expect("set-cookie");

        assert!(set_cookie.starts_with(&format!("{DEV_SESSION_COOKIE_NAME}=")));
        assert!(set_cookie.contains("v1."));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Strict"));
        assert!(set_cookie.contains("Path=/"));
        assert!(!session.value().is_empty());
    }

    #[test]
    fn rejects_forged_dev_session_cookie() {
        let forged = format!("deve_dev_session=v1.forged.{}", "0".repeat(64));
        let session = resolve_from_cookie_header(Some(&forged), SECRET);

        assert_ne!(session.value(), "forged");
        assert!(session.set_cookie().is_some());
    }

    #[test]
    fn dev_session_cookie_is_bound_to_signing_secret() {
        let cookie = cookie_header_for_test(SECRET, "abc_123");
        let rotated_secret = "rotated_test_secret_at_least_32_bytes!";
        let session = resolve_from_cookie_header(Some(&cookie), rotated_secret);

        assert_ne!(session.value(), "abc_123");
        assert!(session.set_cookie().is_some());
    }
}
