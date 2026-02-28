use leptos::prelude::*;

use crate::components::search_box::SearchUiMode;

pub fn footer(ui_mode: Signal<SearchUiMode>) -> impl IntoView {
    view! {
        {move || match ui_mode.get() {
            SearchUiMode::Sheet => view! {
                <div class="bg-sidebar px-4 py-2 border-t border-default flex justify-end items-center text-[11px] text-muted">
                    <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Esc</kbd> to close</span>
                </div>
            }
            .into_any(),
            SearchUiMode::Overlay => view! {
                <div class="bg-sidebar px-4 py-2 border-t border-default flex justify-between items-center text-xs text-muted">
                    <div class="flex gap-4">
                        <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Up/Down</kbd> to navigate</span>
                        <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Enter</kbd> to select</span>
                    </div>
                    <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Esc</kbd> to close</span>
                </div>
            }
            .into_any(),
        }}
    }
}
