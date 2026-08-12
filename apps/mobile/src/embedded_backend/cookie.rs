//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!

use std::fmt;

#[cfg(not(target_os = "android"))]
use tauri::webview::Cookie;
#[cfg(not(target_os = "android"))]
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

    #[cfg(any(target_os = "android", test))]
    pub(super) fn android_install_url(&self) -> String {
        // Android CookieManager applies normal Set-Cookie validation to this
        // URL. Use the secure loopback origin so the required
        // Secure/SameSite=None cookie is accepted.
        format!("https://{}/", self.domain)
    }

    #[cfg(any(target_os = "android", test))]
    pub(super) fn android_verification_url(&self) -> String {
        // CookieManager::getCookie applies the request URL's scheme and cookie
        // security attributes. Verify retention against the same secure origin
        // used for installation; the post-reload auth probe separately proves
        // that the bundled HTTP loopback page can use the session.
        self.android_install_url()
    }

    #[cfg(any(target_os = "android", test))]
    pub(super) fn android_set_cookie_value(&self) -> String {
        format!(
            "{}={}; Path={}; HttpOnly; Secure; SameSite={}",
            self.name, self.value, self.path, self.same_site
        )
    }
}

#[cfg(not(target_os = "android"))]
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

#[cfg(not(target_os = "android"))]
fn tauri_same_site_from_native_session(same_site: &str) -> SameSite {
    match same_site.to_ascii_lowercase().as_str() {
        "none" => SameSite::None,
        "lax" => SameSite::Lax,
        _ => SameSite::Strict,
    }
}

#[cfg(all(not(target_os = "android"), mobile))]
pub(super) async fn install_native_session_cookie_confirmed<R: tauri::Runtime>(
    webview: &tauri::WebviewWindow<R>,
    cookie: &MobileNativeSessionCookie,
) -> Result<(), String> {
    webview
        .set_cookie(tauri_cookie_from_native_session(cookie))
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "android")]
pub(super) async fn install_native_session_cookie_confirmed<R: tauri::Runtime>(
    webview: &tauri::WebviewWindow<R>,
    cookie: &MobileNativeSessionCookie,
) -> Result<(), String> {
    super::android_cookie::install_native_session_cookie_confirmed(webview, cookie).await
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

    #[test]
    fn mobile_embedded_backend_android_native_session_cookie_retention_uses_same_secure_install_origin()
     {
        let cookie = MobileNativeSessionCookie::from_set_cookie(
            "token=cookie-value; Path=/; HttpOnly; Secure; SameSite=None",
            "127.0.0.1",
        )
        .expect("cookie");

        assert_eq!(cookie.android_install_url(), "https://127.0.0.1/");
        assert_eq!(cookie.android_verification_url(), "https://127.0.0.1/");
        assert_eq!(
            cookie.android_verification_url(),
            cookie.android_install_url()
        );
        let value = cookie.android_set_cookie_value();
        assert!(value.starts_with("token=cookie-value; Path=/"));
        assert!(!value.to_ascii_lowercase().contains("domain="));
    }

    #[test]
    fn android_native_session_cookie_waits_for_platform_completion_before_verification() {
        let kotlin = include_str!(
            "../../gen/android/app/src/main/java/dev/deve/notebook/mobile/MainActivity.kt"
        );
        let set_cookie = kotlin
            .find("manager.setCookie(installUrl, setCookie) { accepted ->")
            .expect("callback-confirmed setCookie overload");
        let get_cookie = kotlin[set_cookie..]
            .find("manager.getCookie(verificationUrl)")
            .map(|offset| set_cookie + offset)
            .expect("exact retention verification inside callback");
        let flush = kotlin[get_cookie..]
            .find("manager.flush()")
            .map(|offset| get_cookie + offset)
            .expect("flush after exact retention verification");

        assert!(set_cookie < get_cookie);
        assert!(get_cookie < flush);
        assert!(kotlin.contains("nativeSessionCookieInstallCompleted(requestId, completion)"));
        for constant in [
            "NATIVE_COOKIE_RETAINED = 1",
            "NATIVE_COOKIE_REJECTED = 2",
            "NATIVE_COOKIE_NOT_RETAINED = 3",
            "NATIVE_COOKIE_VERIFICATION_FAILED = 4",
            "NATIVE_COOKIE_SETUP_FAILED = 5",
        ] {
            assert!(
                kotlin.contains(constant),
                "missing platform code: {constant}"
            );
        }

        let rust = include_str!("android_cookie.rs");
        assert!(rust.contains("ANDROID_COOKIE_CALLBACK_TIMEOUT"));
        assert!(rust.contains(".await_completion(receiver, ANDROID_COOKIE_CALLBACK_TIMEOUT)"));
        assert!(rust.contains("env.exception_clear()"));
        assert!(!rust.contains("(Ljava/lang/String;Ljava/lang/String;)V"));

        let callback = include_str!("android_cookie_callback.rs");
        assert!(callback.contains("android_native_cookie_callback_timeout"));
        assert!(callback.contains("android_native_cookie_callback_channel_closed"));

        let shrinker = include_str!("../../gen/android/app/proguard-rules.pro");
        assert!(kotlin.contains("android_native_cookie_retained"));
        assert!(shrinker.contains(
            "public boolean installNativeSessionCookie(long, android.webkit.WebView, java.lang.String, java.lang.String, java.lang.String);"
        ));
        assert!(
            shrinker
                .contains("private native void nativeSessionCookieInstallCompleted(long, int);")
        );
    }
}
