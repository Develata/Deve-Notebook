//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 11_ui_design_01_web#web-layout-persistence
//!
use crate::components::search_box::types::{SearchAction, SearchResult};

pub(super) fn is_mobile() -> bool {
    window_width().map(|w| w <= 768).unwrap_or(false)
}

pub(super) fn is_group(item: &SearchResult) -> bool {
    matches!(item.action, SearchAction::Noop) && item.detail.as_deref() == Some("Group")
}

pub(super) fn is_error(item: &SearchResult) -> bool {
    matches!(item.action, SearchAction::Noop) && item.detail.as_deref() == Some("Error")
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
