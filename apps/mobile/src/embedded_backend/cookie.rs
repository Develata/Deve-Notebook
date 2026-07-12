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
    fn android_install_url(&self) -> String {
        // Android CookieManager applies normal Set-Cookie validation to this
        // URL. Use the secure loopback origin for installation so the required
        // Secure/SameSite=None cookie is accepted; the host-only cookie remains
        // available to Chromium's potentially-trustworthy HTTP loopback origin.
        format!("https://{}/", self.domain)
    }

    #[cfg(any(target_os = "android", test))]
    fn android_verification_url(&self) -> String {
        format!("http://{}/", self.domain)
    }

    #[cfg(any(target_os = "android", test))]
    fn android_set_cookie_value(&self) -> String {
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
    let install_url = cookie.android_install_url();
    let verification_url = cookie.android_verification_url();
    let set_cookie = cookie.android_set_cookie_value();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    webview
        .with_webview(move |platform| {
            platform
                .jni_handle()
                .exec(move |env, _activity, android_webview| {
                    let result = install_android_cookie(
                        env,
                        android_webview,
                        &install_url,
                        &verification_url,
                        &set_cookie,
                    )
                    .map_err(|error| error.to_string());
                    let _ = sender.send(result);
                });
        })
        .map_err(|error| error.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(2), receiver)
        .await
        .map_err(|_| "Android native session cookie install timed out".to_string())?
        .map_err(|_| "Android native session cookie installer stopped".to_string())?
}

#[cfg(target_os = "android")]
#[derive(Debug, thiserror::Error)]
enum AndroidCookieInstallError {
    #[error("Android CookieManager JNI call failed: {0}")]
    Jni(#[from] jni::errors::Error),
    #[error("Android CookieManager did not retain the native session cookie")]
    NotRetained,
}

#[cfg(target_os = "android")]
fn install_android_cookie(
    env: &mut jni::JNIEnv<'_>,
    android_webview: &jni::objects::JObject<'_>,
    install_url: &str,
    verification_url: &str,
    set_cookie: &str,
) -> Result<(), AndroidCookieInstallError> {
    use jni::objects::{JObject, JString, JValue};

    let cookie_manager_class = env.find_class("android/webkit/CookieManager")?;
    let cookie_manager = env
        .call_static_method(
            cookie_manager_class,
            "getInstance",
            "()Landroid/webkit/CookieManager;",
            &[],
        )?
        .l()?;
    env.call_method(
        &cookie_manager,
        "setAcceptCookie",
        "(Z)V",
        &[JValue::Bool(1)],
    )?;
    env.call_method(
        &cookie_manager,
        "setAcceptThirdPartyCookies",
        "(Landroid/webkit/WebView;Z)V",
        &[JValue::Object(android_webview), JValue::Bool(1)],
    )?;

    let expected_cookie_pair = set_cookie
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    let install_url = JObject::from(env.new_string(install_url)?);
    let set_cookie = JObject::from(env.new_string(set_cookie)?);
    env.call_method(
        &cookie_manager,
        "setCookie",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValue::Object(&install_url), JValue::Object(&set_cookie)],
    )?;
    env.call_method(&cookie_manager, "flush", "()V", &[])?;
    let verification_url = JObject::from(env.new_string(verification_url)?);
    let installed = env
        .call_method(
            &cookie_manager,
            "getCookie",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JValue::Object(&verification_url)],
        )?
        .l()?;
    if installed.is_null() {
        return Err(AndroidCookieInstallError::NotRetained);
    }
    let installed = JString::from(installed);
    let installed = env.get_string(&installed)?;
    let installed = installed.to_string_lossy();
    if !installed
        .split(';')
        .map(str::trim)
        .any(|cookie| cookie == expected_cookie_pair)
    {
        return Err(AndroidCookieInstallError::NotRetained);
    }
    Ok(())
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
    fn android_native_session_cookie_is_host_only_for_loopback_ip() {
        let cookie = MobileNativeSessionCookie::from_set_cookie(
            "token=cookie-value; Path=/; HttpOnly; Secure; SameSite=None",
            "127.0.0.1",
        )
        .expect("cookie");

        assert_eq!(cookie.android_install_url(), "https://127.0.0.1/");
        assert_eq!(cookie.android_verification_url(), "http://127.0.0.1/");
        let value = cookie.android_set_cookie_value();
        assert!(value.starts_with("token=cookie-value; Path=/"));
        assert!(!value.to_ascii_lowercase().contains("domain="));
    }
}
