use crate::components::search_box::types::SearchResult;
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::KeyboardEvent;

use super::execute::execute_action;
use super::selection::next_selectable_index;

#[allow(clippy::too_many_arguments)]
pub fn build_keydown_handler(
    show: Signal<bool>,
    query: Signal<String>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    providers_results: Memo<Vec<SearchResult>>,
    active_index: Arc<dyn Fn() -> usize + Send + Sync>,
    input_ref: NodeRef<leptos::html::Input>,
    set_show: WriteSignal<bool>,
    core: CoreState,
    set_recent_move_dirs: WriteSignal<Vec<String>>,
) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    move |ev: KeyboardEvent| {
        let key = ev.key();
        ev.stop_propagation();

        crate::shortcuts::global::handle_search_box_keydown(
            &ev,
            set_show,
            query,
            set_query,
            set_selected_index,
            input_ref,
        );
        if !show.get() {
            return;
        }

        let results = providers_results.get();
        let count = results.len();
        if count == 0 {
            return;
        }

        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                set_selected_index.set(next_selectable_index(&results, active_index(), 1));
            }
            "ArrowUp" => {
                ev.prevent_default();
                set_selected_index.set(next_selectable_index(&results, active_index(), -1));
            }
            "Enter" => {
                ev.prevent_default();
                let idx = active_index();
                if let Some(res) = results.get(idx) {
                    execute_action(
                        &res.action,
                        &core,
                        set_show,
                        set_query,
                        set_selected_index,
                        input_ref,
                        set_recent_move_dirs,
                    );
                }
            }
            _ => {}
        }
    }
}
