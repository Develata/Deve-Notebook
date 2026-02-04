// apps\web\src\components\search_box
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{KeyboardEvent, MouseEvent};

use crate::components::search_box::result_item::result_item;
use crate::components::search_box::types::SearchResult;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};

/// 负责渲染整体遮罩与内部布局。
#[allow(clippy::too_many_arguments)]
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
    set_recent_move_dirs: WriteSignal<Vec<String>>,
) -> impl IntoView {
    let handle_keydown_closure = handle_keydown.clone();
    let active_index_closure = active_index.clone();

    view! {
        <Show when=move || show.get()>
            <div
                class="fixed inset-0 z-[100] font-sans"
                on:click=move |_| set_show.set(false)
            >
                <div
                    class="absolute top-14 left-1/2 -translate-x-1/2 w-full max-w-xl bg-white rounded-lg shadow-xl border border-gray-200 overflow-hidden flex flex-col max-h-[60vh] animate-in fade-in zoom-in-95 duration-100"
                    on:click=move |ev: MouseEvent| ev.stop_propagation()
                    on:keydown={
                        let handle_keydown_closure = handle_keydown_closure.clone();
                        move |ev| handle_keydown_closure(ev)
                    }
                >
                    {header(
                        query,
                        set_query,
                        set_selected_index,
                        placeholder_text,
                        input_ref,
                    )}
                    {results_panel(
                        providers_results,
                        selected_index,
                        set_selected_index,
                        active_index_closure.clone(),
                        set_show,
                        set_query,
                        input_ref,
                        core.clone(),
                        locale,
                        set_recent_move_dirs,
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
#[allow(clippy::too_many_arguments)]
fn results_panel(
    providers_results: Memo<Vec<SearchResult>>,
    selected_index: Signal<usize>,
    set_selected_index: WriteSignal<usize>,
    active_index: Arc<dyn Fn() -> usize + Send + Sync>,
    set_show: WriteSignal<bool>,
    set_query: WriteSignal<String>,
    input_ref: NodeRef<leptos::html::Input>,
    core: CoreState,
    locale: RwSignal<Locale>,
    set_recent_move_dirs: WriteSignal<Vec<String>>,
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
                                            set_query,
                                            input_ref,
                                            core.clone(),
                                            set_recent_move_dirs,
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
