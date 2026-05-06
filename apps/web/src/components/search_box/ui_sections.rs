//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use crate::components::icons::{GitBranch, Search, Terminal};
use crate::components::search_box::SearchUiMode;
use crate::components::search_box::result_item::{SearchResultItemView, result_item};
use crate::components::search_box::types::SearchResult;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use std::sync::Arc;

use super::logic::search_surface_mode;

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
        <div
            data-sheet-drag-handle="1"
            data-deve-search-mode=move || search_surface_mode(&query.get()).as_str()
            class=header_class
        >
            <div class="w-4 h-4 text-muted">
                {search_icon(query)}
            </div>
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

pub struct SearchResultsPanelView {
    pub providers_results: Memo<Vec<SearchResult>>,
    pub selected_index: Signal<usize>,
    pub set_selected_index: WriteSignal<usize>,
    pub active_index: Arc<dyn Fn() -> usize + Send + Sync>,
    pub set_show: WriteSignal<bool>,
    pub set_query: WriteSignal<String>,
    pub input_ref: NodeRef<leptos::html::Input>,
    pub core: CoreState,
    pub locale: RwSignal<Locale>,
    pub set_recent_move_dirs: WriteSignal<Vec<String>>,
    pub results_ref: NodeRef<leptos::html::Div>,
    pub ui_mode: Signal<SearchUiMode>,
}

pub fn results_panel(view: SearchResultsPanelView) -> impl IntoView {
    let SearchResultsPanelView {
        providers_results,
        selected_index,
        set_selected_index,
        active_index,
        set_show,
        set_query,
        input_ref,
        core,
        locale,
        set_recent_move_dirs,
        results_ref,
        ui_mode,
    } = view;
    view! {
        <div
            node_ref=results_ref
            data-sheet-results="1"
            data-deve-search-results-scroll=move || search_results_scroll_marker(ui_mode.get())
            class="overflow-y-auto p-2"
        >
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
                                        result_item(SearchResultItemView {
                                            idx,
                                            item,
                                            is_sel,
                                            selected_index,
                                            set_selected_index,
                                            set_show,
                                            set_query,
                                            input_ref,
                                            core: core.clone(),
                                            set_recent_move_dirs,
                                        })
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

pub(super) fn search_results_scroll_marker(mode: SearchUiMode) -> Option<&'static str> {
    match mode {
        SearchUiMode::Sheet => Some("isolated"),
        SearchUiMode::Overlay => None,
    }
}

fn search_icon(query: Signal<String>) -> impl IntoView {
    move || {
        if query.get().starts_with('>') {
            view! { <Terminal class="w-4 h-4"/> }.into_any()
        } else if query.get().starts_with('@') {
            view! { <GitBranch class="w-4 h-4"/> }.into_any()
        } else {
            view! { <Search class="w-4 h-4"/> }.into_any()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::search_results_scroll_marker;
    use crate::components::search_box::SearchUiMode;

    #[test]
    fn mobile_search_results_scroll_marker_is_sheet_only() {
        assert_eq!(
            search_results_scroll_marker(SearchUiMode::Sheet),
            Some("isolated")
        );
        assert_eq!(search_results_scroll_marker(SearchUiMode::Overlay), None);
    }
}
