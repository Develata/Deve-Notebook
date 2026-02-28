use crate::components::search_box::SearchUiMode;
use crate::components::search_box::result_item::result_item;
use crate::components::search_box::types::SearchResult;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use std::sync::Arc;

pub fn header(
    query: Signal<String>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    placeholder_text: Memo<String>,
    input_ref: NodeRef<leptos::html::Input>,
    ui_mode: Signal<SearchUiMode>,
) -> impl IntoView {
    let header_class = move || match ui_mode.get() {
        SearchUiMode::Sheet => {
            "px-3 py-2 border-b border-default flex items-center gap-2 bg-sidebar"
        }
        SearchUiMode::Overlay => "p-3 border-b border-default flex items-center gap-2 bg-sidebar",
    };
    view! {
        <div data-sheet-drag-handle="1" class=header_class>
            <svg class="w-4 h-4 text-muted" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                {search_icon(query)}
            </svg>
            <input
                name="search-query"
                node_ref=input_ref
                type="text"
                class="flex-1 outline-none text-sm bg-transparent text-primary placeholder:text-muted"
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

#[allow(clippy::too_many_arguments)]
pub fn results_panel(
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
    results_ref: NodeRef<leptos::html::Div>,
) -> impl IntoView {
    view! {
        <div node_ref=results_ref data-sheet-results="1" class="overflow-y-auto p-2">
            {
                let core = core.clone();
                move || {
                    let res = providers_results.get();
                    let core = core.clone();
                    if res.is_empty() {
                        view! {
                            <div class="p-4 text-center text-muted text-sm">
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
