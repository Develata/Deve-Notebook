// apps/web/src/components/search_box/result_item.rs
//! # 搜索结果项渲染组件
//!
//! 处理单条搜索结果的渲染和交互逻辑。

use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::components::search_box::types::{SearchAction, SearchResult};
use crate::hooks::use_core::CoreState;

/// 单条结果项，支持鼠标与键盘操作。
pub fn result_item(
    idx: usize,
    item: SearchResult,
    is_sel: bool,
    selected_index: Signal<usize>,
    set_selected_index: WriteSignal<usize>,
    set_show: WriteSignal<bool>,
    core: CoreState,
) -> impl IntoView {
    let detail_icon = item.detail.clone();
    let detail_text = item.detail.clone();
    let detail_text_cond = detail_text.clone();

    view! {
        <button
            class=format!(
                "w-full text-left px-4 py-3 rounded-lg flex items-center gap-3 group transition-colors {}",
                if is_sel { "bg-blue-50 text-blue-700" } else { "text-gray-700 hover:bg-gray-50" }
            )
            on:click=move |_| {
                let action = item.action.clone();
                let core_clone = core.clone();
                request_animation_frame(move || {
                    match action {
                        SearchAction::OpenDoc(id) => {
                            core_clone.on_doc_select.run(id);
                            set_show.set(false);
                        }
                        SearchAction::RunCommand(cmd) => {
                            cmd.action.run(());
                        }
                        SearchAction::SwitchBranch(branch) => {
                            if branch == "Local (Master)" {
                                core_clone.on_switch_branch.run(None);
                            } else {
                                core_clone.on_switch_branch.run(Some(branch));
                            }
                            set_show.set(false);
                        }
                        SearchAction::CreateDoc(path) => {
                            let normalized = path.replace('\\', "/");
                            let target = if normalized.ends_with(".md") {
                                normalized.clone()
                            } else {
                                format!("{}.md", normalized)
                            };

                            core_clone.on_doc_create.run(target);
                            set_show.set(false);
                        }
                    }
                });
            }
            on:mousemove=move |_: MouseEvent| {
                if selected_index.get_untracked() != idx {
                    set_selected_index.set(idx);
                }
            }
        >
            {item_icon(is_sel, detail_icon)}
            {item_content(item.title.clone(), detail_text_cond, detail_text)}
            {selection_arrow(is_sel)}
        </button>
    }
}

fn item_icon(is_sel: bool, detail: Option<String>) -> impl IntoView {
    view! {
        <div class=format!("flex-none {}", if is_sel { "text-blue-500" } else { "text-gray-400" })>
            <Show when=move || detail.as_deref() == Some("Command") fallback=|| view! {
                <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
                </svg>
            }>
                <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                </svg>
            </Show>
        </div>
    }
}

fn item_content(
    title: String,
    detail_cond: Option<String>,
    detail_text: Option<String>,
) -> impl IntoView {
    view! {
        <div class="flex-1 truncate flex flex-col items-start gap-0.5">
            <span class="font-medium">{title}</span>
            <Show when=move || detail_cond.is_some()>
                <span class="text-xs opacity-60 font-mono">
                    {detail_text.clone().unwrap()}
                </span>
            </Show>
        </div>
    }
}

fn selection_arrow(is_sel: bool) -> impl IntoView {
    view! {
        <Show when=move || is_sel>
            <svg class="w-4 h-4 text-blue-500 opacity-0 group-hover:opacity-100 transition-opacity" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
            </svg>
        </Show>
    }
}
