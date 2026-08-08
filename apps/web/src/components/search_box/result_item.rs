// apps/web/src/components/search_box/result_item.rs
//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! # 搜索结果项渲染组件
//!
//! 处理单条搜索结果的渲染和交互逻辑。

use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::components::search_box::logic;
use crate::components::search_box::runtime::SearchRuntime;
use crate::components::search_box::types::{SearchAction, SearchResult};
use crate::components::touch_feedback::interactive_item_state_class;

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
    pub runtime: SearchRuntime,
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
        runtime,
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
    let action_marker = search_action_marker(&item.action);
    let create_target_marker = search_action_create_target(&item.action);
    let doc_id_marker = search_action_doc_id(&item.action);
    let title_marker = item.title.clone();

    view! {
        <button
            type="button"
            class=format!(
                "{} {}",
                base,
                interactive_item_state_class(is_sel, is_selectable)
            )
            data-deve-search-result="true"
            data-deve-search-result-action=action_marker
            data-deve-search-result-create-target=create_target_marker
            data-deve-search-result-doc-id=doc_id_marker
            data-deve-search-result-title=title_marker
            on:click=move |_| {
                if !is_selectable {
                    return;
                }
                let action = item.action.clone();
                let runtime_clone = runtime.clone();
                request_animation_frame(move || {
                    logic::execute_action(
                        &action,
                        &runtime_clone,
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

fn search_action_marker(action: &SearchAction) -> &'static str {
    match action {
        SearchAction::OpenDoc(_) => "open-doc",
        SearchAction::RunCommand(_) => "run-command",
        SearchAction::SwitchBranch(_) => "switch-branch",
        SearchAction::CreateDoc(_) => "create-doc",
        SearchAction::FileOp(_) => "file-op",
        SearchAction::InsertQuery(_) => "insert-query",
        SearchAction::Noop => "noop",
    }
}

fn search_action_create_target(action: &SearchAction) -> Option<String> {
    match action {
        SearchAction::CreateDoc(path) => Some(path.clone()),
        _ => None,
    }
}

fn search_action_doc_id(action: &SearchAction) -> Option<String> {
    match action {
        SearchAction::OpenDoc(doc_id) => Some(doc_id.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{search_action_create_target, search_action_doc_id};
    use crate::components::search_box::types::SearchAction;
    use deve_core::models::DocId;

    #[test]
    fn search_action_identity_markers_follow_typed_action_payloads() {
        let doc_id = DocId::from_u128(7);
        assert_eq!(
            search_action_create_target(&SearchAction::CreateDoc("notes/exact.md".into())),
            Some("notes/exact.md".into())
        );
        assert_eq!(
            search_action_doc_id(&SearchAction::OpenDoc(doc_id)),
            Some(doc_id.to_string())
        );
        assert_eq!(
            search_action_create_target(&SearchAction::OpenDoc(doc_id)),
            None
        );
        assert_eq!(
            search_action_doc_id(&SearchAction::CreateDoc("notes/exact.md".into())),
            None
        );
    }
}
