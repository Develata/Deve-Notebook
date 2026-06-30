//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::search_box::SearchUiMode;
use leptos::prelude::*;

pub(super) fn panel_class(mode: SearchUiMode) -> &'static str {
    match mode {
        SearchUiMode::Sheet => {
            "absolute top-0 left-0 right-0 bg-panel rounded-b-2xl shadow-xl border border-default overflow-hidden flex flex-col max-h-[72vh] animate-in fade-in slide-in-from-top-4 duration-200 ease-out"
        }
        SearchUiMode::Overlay => {
            "absolute top-14 left-1/2 -translate-x-1/2 w-full max-w-xl bg-panel rounded-lg shadow-xl border border-default overflow-hidden flex flex-col max-h-[60vh] animate-in fade-in zoom-in-95 duration-200 ease-out"
        }
    }
}

pub(super) fn panel_style(
    mode: SearchUiMode,
    sheet_drag_offset: i32,
    sheet_dragging: bool,
) -> String {
    match mode {
        SearchUiMode::Sheet => {
            let transition = if sheet_dragging {
                "none"
            } else {
                "transform 200ms ease-out"
            };
            format!(
                "padding-top: env(safe-area-inset-top); transform: translateY({}px); transition: {};",
                sheet_drag_offset, transition
            )
        }
        SearchUiMode::Overlay => String::new(),
    }
}

pub(super) fn sheet_position(mode: SearchUiMode) -> Option<&'static str> {
    match mode {
        SearchUiMode::Sheet => Some("top"),
        SearchUiMode::Overlay => None,
    }
}

pub(super) fn backdrop_class(mode: SearchUiMode) -> &'static str {
    match mode {
        SearchUiMode::Sheet => {
            "fixed inset-0 z-[var(--z-modal)] font-sans bg-black/20 backdrop-blur-[1px]"
        }
        SearchUiMode::Overlay => "fixed inset-0 z-[var(--z-modal)] font-sans",
    }
}

pub(super) fn drag_handle(mode: SearchUiMode) -> impl IntoView {
    match mode {
        SearchUiMode::Sheet => view! {
            <div
                data-sheet-drag-handle="1"
                data-deve-search-sheet-handle="top"
                class="flex justify-center py-2"
            >
                <div class="w-10 h-1.5 rounded-full bg-active"></div>
            </div>
        }
        .into_any(),
        SearchUiMode::Overlay => view! {}.into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::{backdrop_class, panel_class, panel_style, sheet_position};
    use crate::components::search_box::SearchUiMode;

    #[test]
    fn mobile_search_sheet_is_positioned_at_top() {
        let class = panel_class(SearchUiMode::Sheet);
        assert!(class.contains("top-0"));
        assert!(!class.contains("bottom-0"));
        assert_eq!(sheet_position(SearchUiMode::Sheet), Some("top"));
        assert_eq!(sheet_position(SearchUiMode::Overlay), None);
    }

    #[test]
    fn mobile_search_sheet_drag_style_moves_upward() {
        assert_eq!(
            panel_style(SearchUiMode::Sheet, -64, true),
            "padding-top: env(safe-area-inset-top); transform: translateY(-64px); transition: none;"
        );
        assert!(panel_style(SearchUiMode::Overlay, -64, true).is_empty());
    }

    #[test]
    fn search_sheet_uses_modal_layer_above_mobile_drawers() {
        assert!(backdrop_class(SearchUiMode::Sheet).contains("z-[var(--z-modal)]"));
        assert!(backdrop_class(SearchUiMode::Overlay).contains("z-[var(--z-modal)]"));
    }
}
