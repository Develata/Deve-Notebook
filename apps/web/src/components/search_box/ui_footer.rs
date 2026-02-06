use leptos::prelude::*;

use crate::components::search_box::SearchUiMode;

pub fn footer(ui_mode: Signal<SearchUiMode>) -> impl IntoView {
    view! {
        {move || match ui_mode.get() {
            SearchUiMode::Sheet => view! {
                <div class="bg-gray-50 px-4 py-2 border-t border-gray-100 flex justify-end items-center text-[11px] text-gray-500">
                    <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Esc</kbd> to close</span>
                </div>
            }
            .into_any(),
            SearchUiMode::Overlay => view! {
                <div class="bg-gray-50 px-4 py-2 border-t border-gray-100 flex justify-between items-center text-xs text-gray-500">
                    <div class="flex gap-4">
                        <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Up/Down</kbd> to navigate</span>
                        <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Enter</kbd> to select</span>
                    </div>
                    <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Esc</kbd> to close</span>
                </div>
            }
            .into_any(),
        }}
    }
}
