use super::{loading_progress_compact, storage_limited_read_only, toggle_status_details};
use crate::i18n::Locale;

#[test]
fn mobile_i18n_bottom_bar_toggle_copy_has_facade_key() {
    assert_eq!(toggle_status_details(Locale::En), "Toggle status details");
}

#[test]
fn degraded_storage_banner_is_localized_by_locale() {
    assert_eq!(
        storage_limited_read_only(Locale::En, "IndexedDB=false"),
        "Storage limited (IndexedDB=false); read-only mode is active"
    );
    assert_eq!(
        storage_limited_read_only(Locale::Zh, "IndexedDB=false"),
        "存储受限（IndexedDB=false），当前处于只读模式"
    );
}

#[test]
fn mobile_compact_loading_progress_is_localized() {
    assert_eq!(loading_progress_compact(Locale::En, 2, 5), "Load 2/5");
    assert_eq!(loading_progress_compact(Locale::Zh, 2, 5), "加载 2/5");
}
