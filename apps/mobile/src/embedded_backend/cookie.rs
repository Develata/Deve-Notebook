use std::fmt;

use tauri::webview::Cookie;
use tauri::webview::cookie::SameSite;

use super::MobileEmbeddedBackendError;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct MobileNativeSessionCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    http_only: bool,
    same_site: String,
}

impl fmt::Debug for MobileNativeSessionCookie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MobileNativeSessionCookie")
            .field("name", &self.name)
            .field("value", &"<redacted>")
            .field("domain", &self.domain)
            .field("path", &self.path)
            .field("secure", &self.secure)
            .field("http_only", &self.http_only)
            .field("same_site", &self.same_site)
            .finish()
    }
}

impl MobileNativeSessionCookie {
    pub(super) fn from_set_cookie(
        set_cookie: &str,
        domain: &str,
    ) -> Result<Self, MobileEmbeddedBackendError> {
        let mut parts = set_cookie.split(';').map(str::trim);
        let Some(name_value) = parts.next() else {
            return Err(MobileEmbeddedBackendError::NativeSessionCookieInvalid);
        };
        let Some((name, value)) = name_value.split_once('=') else {
            return Err(MobileEmbeddedBackendError::NativeSessionCookieInvalid);
        };
        let name = name.trim();
        let value = value.trim();
        if name != "token" || value.is_empty() || !matches!(domain, "127.0.0.1" | "localhost") {
            return Err(MobileEmbeddedBackendError::NativeSessionCookieInvalid);
        }

        let mut cookie = Self {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: String::new(),
        };
        for attr in parts {
            let lower = attr.to_ascii_lowercase();
            if lower == "httponly" {
                cookie.http_only = true;
            } else if lower == "secure" {
                cookie.secure = true;
            } else if let Some((key, value)) = attr.split_once('=') {
                if key.eq_ignore_ascii_case("path") {
                    cookie.path = value.trim().to_string();
                } else if key.eq_ignore_ascii_case("samesite") {
                    cookie.same_site = value.trim().to_string();
                }
            }
        }
        if !cookie.http_only || !cookie.secure || !cookie.same_site.eq_ignore_ascii_case("none") {
            return Err(MobileEmbeddedBackendError::NativeSessionCookieInvalid);
        }
        Ok(cookie)
    }

    pub(super) fn request_cookie_header(&self) -> String {
        format!("{}={}", self.name, self.value)
    }

    pub(super) fn has_value(&self) -> bool {
        !self.value.is_empty()
    }
}

pub(super) fn tauri_cookie_from_native_session(
    cookie: &MobileNativeSessionCookie,
) -> Cookie<'static> {
    Cookie::build((cookie.name.clone(), cookie.value.clone()))
        .domain(cookie.domain.clone())
        .path(cookie.path.clone())
        .http_only(cookie.http_only)
        .same_site(tauri_same_site_from_native_session(&cookie.same_site))
        .secure(cookie.secure)
        .build()
}

fn tauri_same_site_from_native_session(same_site: &str) -> SameSite {
    match same_site.to_ascii_lowercase().as_str() {
        "none" => SameSite::None,
        "lax" => SameSite::Lax,
        _ => SameSite::Strict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_native_session_cookie_requires_http_only_secure_same_site_none() {
        let cookie = MobileNativeSessionCookie::from_set_cookie(
            "token=cookie-value; Path=/; HttpOnly; Secure; SameSite=None",
            "127.0.0.1",
        )
        .expect("cookie");

        assert_eq!(cookie.request_cookie_header(), "token=cookie-value");
        assert_eq!(
            tauri_same_site_from_native_session(&cookie.same_site),
            SameSite::None
        );

        assert!(matches!(
            MobileNativeSessionCookie::from_set_cookie("token=cookie-value; Path=/", "127.0.0.1"),
            Err(MobileEmbeddedBackendError::NativeSessionCookieInvalid)
        ));
    }

    #[test]
    fn mobile_native_session_cookie_debug_redacts_value() {
        let cookie = MobileNativeSessionCookie::from_set_cookie(
            "token=cookie-value; Path=/; HttpOnly; Secure; SameSite=None",
            "127.0.0.1",
        )
        .expect("cookie");

        let debug = format!("{cookie:?}");
        assert!(!debug.contains("cookie-value"));
        assert!(debug.contains("<redacted>"));
    }
}
