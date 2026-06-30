//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::search_box::types::{SearchResult, SearchResultRole};

pub(super) fn is_mobile() -> bool {
    window_width().map(|w| w <= 768).unwrap_or(false)
}

pub(super) fn is_group(item: &SearchResult) -> bool {
    item.role == SearchResultRole::Group
}

pub(super) fn is_error(item: &SearchResult) -> bool {
    item.role == SearchResultRole::Error
}

pub(super) fn base_row_class(is_mobile: bool) -> &'static str {
    if is_mobile {
        "w-full text-left px-3 py-2 rounded-lg flex items-center gap-2 group transition-colors active:bg-hover"
    } else {
        "w-full text-left px-4 py-3 rounded-lg flex items-center gap-3 group transition-colors active:bg-hover"
    }
}

fn window_width() -> Option<i32> {
    let window = web_sys::window()?;
    let width = window.inner_width().ok()?.as_f64()?;
    Some(width as i32)
}

#[cfg(test)]
mod tests {
    use super::{is_error, is_group};
    use crate::components::search_box::types::{SearchAction, SearchResult, SearchResultRole};
    use crate::i18n::{Locale, t};

    fn noop_result(id: &str, detail: &'static str, role: SearchResultRole) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            title: "row".to_string(),
            detail: Some(detail.to_string()),
            role,
            score: 0.0,
            action: SearchAction::Noop,
        }
    }

    #[test]
    fn localized_error_detail_still_renders_as_error_row() {
        let item = noop_result(
            "fileop-error",
            t::search::error_detail(Locale::Zh),
            SearchResultRole::Error,
        );

        assert!(is_error(&item));
    }

    #[test]
    fn localized_group_detail_still_renders_as_group_row() {
        let item = noop_result(
            "group-recent",
            t::search::group_detail(Locale::Zh),
            SearchResultRole::Group,
        );

        assert!(is_group(&item));
    }
}
