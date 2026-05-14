// apps/web/src/components/search_box/result_item.rs
//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 08_ui_design_01_web#web-layout-persistence
//!
//! # 搜索结果项渲染组件
//!
//! 处理单条搜索结果的渲染和交互逻辑。

use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::components::search_box::logic;
use crate::components::search_box::types::SearchResult;
use crate::components::touch_feedback::interactive_item_state_class;
use crate::hooks::use_core::CoreState;

mod sections;
mod state;

pub struct SearchResultItemView {
    pub idx: usize,
    pub item: SearchResult,
    pub is_sel: bool,
    pub selected_index: Signal<usize>,
    pub set_selected_index: WriteSignal<usize>,
    pub set_show: WriteSignal<bool>,
    pub set_query: WriteSignal<String>,
    pub input_ref: NodeRef<leptos::html::Input>,
    pub core: CoreState,
    pub set_recent_move_dirs: WriteSignal<Vec<String>>,
}

/// 单条结果项，支持鼠标与键盘操作。
pub fn result_item(view: SearchResultItemView) -> impl IntoView {
    let SearchResultItemView {
        idx,
        item,
        is_sel,
        selected_index,
        set_selected_index,
        set_show,
        set_query,
        input_ref,
        core,
        set_recent_move_dirs,
    } = view;
    let is_mobile = state::is_mobile();
    let detail_text = item.detail.clone();
    let is_group = state::is_group(&item);
    let is_error = state::is_error(&item);
    let is_selectable = logic::is_selectable(Some(&item));

    if is_group {
        return sections::group_row(item.title, is_mobile).into_any();
    }

    if is_error {
        return sections::error_row(item.title, is_mobile).into_any();
    }

    let base = state::base_row_class(is_mobile);

    let action_clone = item.action.clone();
    let detail_clone = item.detail.clone();

    view! {
        <button
            class=format!(
                "{} {}",
                base,
                interactive_item_state_class(is_sel, is_selectable)
            )
            on:click=move |_| {
                if !is_selectable {
                    return;
                }
                let action = item.action.clone();
                let core_clone = core.clone();
                request_animation_frame(move || {
                    logic::execute_action(
                        &action,
                        &core_clone,
                        set_show,
                        set_query,
                        set_selected_index,
                        input_ref,
                        set_recent_move_dirs,
                    );
                });
            }
            on:mousemove=move |_: MouseEvent| {
                if is_selectable && selected_index.get_untracked() != idx {
                    set_selected_index.set(idx);
                }
            }
            on:touchstart=move |_| {
                if is_selectable && selected_index.get_untracked() != idx {
                    set_selected_index.set(idx);
                }
            }
        >
            {move || if is_mobile {
                view! {}.into_any()
            } else {
                sections::item_icon(is_sel, action_clone.clone(), detail_clone.clone())
                    .into_any()
            }}
            {sections::item_content(item.title.clone(), detail_text, is_mobile)}
            {move || if is_mobile {
                view! {}.into_any()
            } else {
                sections::selection_arrow(is_sel).into_any()
            }}
        </button>
    }
    .into_any()
}
