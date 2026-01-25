// apps\web\src\components\search_box
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{KeyboardEvent, MouseEvent};

use crate::components::search_box::types::{SearchAction, SearchResult};
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};

/// 负责渲染整体遮罩与内部布局。
pub fn render_overlay(
    show: Signal<bool>,
    set_show: WriteSignal<bool>,
    query: Signal<String>,
    set_query: WriteSignal<String>,
    placeholder_text: Memo<String>,
    handle_keydown: Arc<dyn Fn(KeyboardEvent) + Send + Sync>,
    providers_results: Memo<Vec<SearchResult>>,
    selected_index: Signal<usize>,
    set_selected_index: WriteSignal<usize>,
    active_index: Arc<dyn Fn() -> usize + Send + Sync>,
    input_ref: NodeRef<leptos::html::Input>,
    core: CoreState,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let handle_keydown_closure = handle_keydown.clone();
    let active_index_closure = active_index.clone();

    view! {
        <Show when=move || show.get()>
            <div
                class="fixed inset-0 z-[60] font-sans"
                on:click=move |_| set_show.set(false)
            >
                <div
                    class="absolute top-2 left-1/2 -translate-x-1/2 w-full max-w-xl bg-white rounded-lg shadow-xl border border-gray-200 overflow-hidden flex flex-col max-h-[60vh] animate-in fade-in zoom-in-95 duration-100"
                    on:click=move |ev: MouseEvent| ev.stop_propagation()
                    on:keydown={
                        let handle_keydown_closure = handle_keydown_closure.clone();
                        move |ev| handle_keydown_closure(ev)
                    }
                >
                    {header(query, set_query, set_selected_index, placeholder_text, input_ref)}
                    {results_panel(
                        providers_results,
                        selected_index,
                        set_selected_index,
                        active_index_closure.clone(),
                        set_show,
                        core.clone(),
                        locale,
                    )}
                    {footer()}
                </div>
            </div>
        </Show>
    }
}

/// 头部搜索框区域。
fn header(
    query: Signal<String>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    placeholder_text: Memo<String>,
    input_ref: NodeRef<leptos::html::Input>,
) -> impl IntoView {
    view! {
        <div class="p-3 border-b border-gray-100 flex items-center gap-3 bg-gray-50/50">
            <svg class="w-4 h-4 text-gray-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                {search_icon(query)}
            </svg>
            <input
                node_ref=input_ref
                type="text"
                class="flex-1 outline-none text-base bg-transparent text-gray-800 placeholder:text-gray-400"
                placeholder=move || placeholder_text.get()
                prop:value=move || query.get()
                on:input=move |ev| {
                    set_query.set(event_target_value(&ev));
                    set_selected_index.set(0);
                }
            />
        </div>
    }
}

/// 根据输入模式切换显示的图标。
fn search_icon(query: Signal<String>) -> impl IntoView {
    move || {
        if query.get().starts_with('>') {
            view! { <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" /> }
                .into_any()
        } else if query.get().starts_with('@') {
            view! { <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/></svg> }
                .into_any()
        } else {
            view! { <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /> }
                .into_any()
        }
    }
}

/// 列表区域，包含空态与结果列表。
fn results_panel(
    providers_results: Memo<Vec<SearchResult>>,
    selected_index: Signal<usize>,
    set_selected_index: WriteSignal<usize>,
    active_index: Arc<dyn Fn() -> usize + Send + Sync>,
    set_show: WriteSignal<bool>,
    core: CoreState,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        <div class="overflow-y-auto p-2">
            {
                let core = core.clone();
                move || {
                    let res = providers_results.get();
                    let core = core.clone();

                    if res.is_empty() {
                        view! {
                            <div class="p-4 text-center text-gray-400 text-sm">
                                {move || t::command_palette::no_results(locale.get())}
                            </div>
                        }
                        .into_any()
                    } else {
                        let idx_sel = active_index.as_ref()();
                        view! {
                            <div class="flex flex-col gap-1">
                                <For
                                    each=move || res.clone().into_iter().enumerate()
                                    key=|(idx, r)| format!("{}-{}", idx, r.id)
                                    children=move |(idx, item)| {
                                        let is_sel = idx == idx_sel;
                                        result_item(
                                            idx,
                                            item,
                                            is_sel,
                                            selected_index,
                                            set_selected_index,
                                            set_show,
                                            core.clone(),
                                        )
                                    }
                                />
                            </div>
                        }
                        .into_any()
                    }
                }
            }
        </div>
    }
}

/// 单条结果项，支持鼠标与键盘操作。
fn result_item(
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
            on:mousemove=move |_| {
                if selected_index.get_untracked() != idx {
                    set_selected_index.set(idx);
                }
            }
        >
            <div class=format!("flex-none {}", if is_sel { "text-blue-500" } else { "text-gray-400" })>
                <Show when=move || detail_icon.as_deref() == Some("Command") fallback=|| view! {
                    <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
                    </svg>
                }>
                    <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                    </svg>
                </Show>
            </div>

            <div class="flex-1 truncate flex flex-col items-start gap-0.5">
                <span class="font-medium">{item.title.clone()}</span>
                <Show when=move || detail_text_cond.is_some()>
                    <span class="text-xs opacity-60 font-mono">
                        {detail_text.clone().unwrap()}
                    </span>
                </Show>
            </div>

            <Show when=move || is_sel>
                <svg class="w-4 h-4 text-blue-500 opacity-0 group-hover:opacity-100 transition-opacity" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
                </svg>
            </Show>
        </button>
    }
}

/// 底部快捷键提示。
fn footer() -> impl IntoView {
    view! {
        <div class="bg-gray-50 px-4 py-2 border-t border-gray-100 flex justify-between items-center text-xs text-gray-500">
            <div class="flex gap-4">
                <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Up/Down</kbd> to navigate</span>
                <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Enter</kbd> to select</span>
            </div>
            <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Esc</kbd> to close</span>
        </div>
    }
}
