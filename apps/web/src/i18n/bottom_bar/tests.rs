use super::{loading_progress_compact, storage_limited_read_only, toggle_status_details};
use crate::i18n::Locale;
use crate::storage::BrowserIdentityBlocker;

#[test]
fn mobile_i18n_bottom_bar_toggle_copy_has_facade_key() {
    assert_eq!(toggle_status_details(Locale::En), "Toggle status details");
}

#[test]
fn degraded_storage_banner_is_localized_by_locale() {
    assert_eq!(
        storage_limited_read_only(Locale::En, BrowserIdentityBlocker::IndexedDbUnavailable),
        "Persistent browser storage is unavailable; read-only mode is active. Allow site storage or use a supported browser."
    );
    assert_eq!(
        storage_limited_read_only(Locale::Zh, BrowserIdentityBlocker::IndexedDbUnavailable),
        "浏览器持久存储不可用，当前保持只读。请允许站点存储或改用受支持的浏览器。"
    );
}

#[test]
fn ed25519_blocker_copy_points_to_browser_or_system_webview_update() {
    assert!(
        storage_limited_read_only(Locale::En, BrowserIdentityBlocker::Ed25519Unavailable)
            .contains("Android System WebView")
    );
    assert!(
        storage_limited_read_only(Locale::Zh, BrowserIdentityBlocker::Ed25519Unavailable)
            .contains("Android System WebView")
    );
}

#[test]
fn mobile_compact_loading_progress_is_localized() {
    assert_eq!(loading_progress_compact(Locale::En, 2, 5), "Load 2/5");
    assert_eq!(loading_progress_compact(Locale::Zh, 2, 5), "加载 2/5");
}
