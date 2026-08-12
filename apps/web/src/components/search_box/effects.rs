// apps\web\src\components\search_box
//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::focus_scope;
use crate::components::search_box::SearchUiMode;
use leptos::prelude::*;

pub struct SearchFocusEffect {
    pub show: Signal<bool>,
    pub mode_signal: Signal<String>,
    pub ui_mode: Signal<SearchUiMode>,
    pub set_query: WriteSignal<String>,
    pub set_selected_index: WriteSignal<usize>,
    pub input_ref: NodeRef<leptos::html::Input>,
    pub close_ref: NodeRef<leptos::html::Button>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchInitialFocus {
    CloseButton,
    QueryInput,
}

fn initial_focus(mode: SearchUiMode) -> SearchInitialFocus {
    match mode {
        SearchUiMode::Sheet => SearchInitialFocus::CloseButton,
        SearchUiMode::Overlay => SearchInitialFocus::QueryInput,
    }
}

/// 管理输入框与编辑器的焦点切换。
pub fn attach_focus_effect(input: SearchFocusEffect) {
    let SearchFocusEffect {
        show,
        mode_signal,
        ui_mode,
        set_query,
        set_selected_index,
        input_ref,
        close_ref,
    } = input;
    let last_show = StoredValue::new_local(show.get_untracked());
    let previous_focus = StoredValue::new_local(None::<web_sys::Element>);

    Effect::new(move |_| {
        let open = show.get();
        let Some(was_open) = last_show.try_get_value() else {
            return;
        };
        if last_show.try_set_value(open).is_some() {
            return;
        }

        if open {
            // 打开时重置查询并聚焦搜索框。
            let raw = mode_signal.get();
            let cursor_pos = raw.chars().take_while(|c| *c != '|').count();
            let cleaned = raw.replacen('|', "", 1);
            let has_cursor = raw.contains('|');
            set_query.set(cleaned);
            set_selected_index.set(0);

            if !was_open
                && previous_focus
                    .try_set_value(focus_scope::active_element())
                    .is_some()
            {
                return;
            }
            let open_mode = ui_mode.get_untracked();
            request_animation_frame(move || match initial_focus(open_mode) {
                SearchInitialFocus::CloseButton => {
                    if let Some(button) = close_ref.try_get_untracked().flatten() {
                        let _ = button.focus();
                    }
                }
                SearchInitialFocus::QueryInput => {
                    if let Some(el) = input_ref.try_get_untracked().flatten() {
                        let _ = el.focus();
                        if has_cursor {
                            let _ = el.set_selection_range(cursor_pos as u32, cursor_pos as u32);
                        }
                    }
                }
            });
        } else if was_open {
            let Some(previous) = previous_focus.try_get_value() else {
                return;
            };
            if previous_focus.try_set_value(None).is_some() {
                return;
            }
            focus_scope::restore_focus_next_frame(previous);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{SearchInitialFocus, initial_focus};
    use crate::components::search_box::SearchUiMode;

    #[test]
    fn search_sheet_initial_focus_does_not_open_the_ime() {
        assert_eq!(
            initial_focus(SearchUiMode::Sheet),
            SearchInitialFocus::CloseButton
        );
        assert_eq!(
            initial_focus(SearchUiMode::Overlay),
            SearchInitialFocus::QueryInput
        );
    }
}
