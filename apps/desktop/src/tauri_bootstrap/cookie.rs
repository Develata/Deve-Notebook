use tauri::webview::Cookie;
use tauri::webview::cookie::SameSite;

use crate::DesktopNativeSessionCookie;

use super::DesktopTauriBootstrapError;

pub(super) fn tauri_cookie_from_native_session(
    cookie: &DesktopNativeSessionCookie,
) -> Cookie<'static> {
    Cookie::build((cookie.name().to_string(), cookie.value().to_string()))
        .domain(cookie.domain().to_string())
        .path(cookie.path().to_string())
        .http_only(cookie.http_only())
        .same_site(tauri_same_site_from_native_session(cookie.same_site()))
        .secure(cookie.secure())
        .build()
}

fn tauri_same_site_from_native_session(same_site: &str) -> SameSite {
    match same_site.to_ascii_lowercase().as_str() {
        "none" => SameSite::None,
        "lax" => SameSite::Lax,
        _ => SameSite::Strict,
    }
}

pub(super) fn validate_tauri_bootstrap_source(
    source: &str,
) -> Result<(), DesktopTauriBootstrapError> {
    let source_lower = source.to_ascii_lowercase();
    for marker in [
        "<script",
        "</script",
        "token",
        "secret",
        "localstorage",
        "location.href",
        "auth_pass",
        "auth_secret",
    ] {
        if source_lower.contains(marker) {
            return Err(DesktopTauriBootstrapError::ForbiddenMaterial { marker });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SameSite, tauri_same_site_from_native_session};

    #[test]
    fn tauri_cookie_mapping_preserves_native_session_same_site_none() {
        assert_eq!(tauri_same_site_from_native_session("None"), SameSite::None);
    }
}
