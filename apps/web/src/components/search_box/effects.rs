// apps\web\src\components\search_box
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// 管理输入框与编辑器的焦点切换。
pub fn attach_focus_effect(
    show: Signal<bool>,
    mode_signal: Signal<String>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    input_ref: NodeRef<leptos::html::Input>,
) {
    Effect::new(move |_| {
        if show.get() {
            // 打开时重置查询并聚焦搜索框。
            let raw = mode_signal.get();
            let cursor_pos = raw.chars().take_while(|c| *c != '|').count();
            let cleaned = raw.replacen('|', "", 1);
            let has_cursor = raw.contains('|');
            set_query.set(cleaned);
            set_selected_index.set(0);

            request_animation_frame(move || {
                if let Some(el) = input_ref.get_untracked() {
                    let _ = el.focus();
                    if has_cursor {
                        let _ = el.set_selection_range(cursor_pos as u32, cursor_pos as u32);
                    }
                }
            });
        } else {
            // 关闭后把焦点交还给编辑器，维持流畅体验。
            request_animation_frame(move || {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Ok(Some(el)) = document.query_selector(".cm-content") {
                            if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                                let _ = html_el.focus();
                            }
                        }
                    }
                }
            });
        }
    });
}
