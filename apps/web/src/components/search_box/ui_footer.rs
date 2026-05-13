//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use leptos::prelude::*;

use crate::components::search_box::SearchUiMode;
use crate::i18n::{Locale, t};

pub fn footer(ui_mode: Signal<SearchUiMode>, locale: RwSignal<Locale>) -> impl IntoView {
    view! {
        {move || match ui_mode.get() {
            SearchUiMode::Sheet => view! {
                <div class="bg-sidebar px-4 py-2 border-t border-default flex justify-end items-center text-[11px] text-muted">
                    <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Esc</kbd> " " {move || t::command_palette::keyboard_close_hint(locale.get())}</span>
                </div>
            }
            .into_any(),
            SearchUiMode::Overlay => view! {
                <div class="bg-sidebar px-4 py-2 border-t border-default flex justify-between items-center text-xs text-muted">
                    <div class="flex gap-4">
                        <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Up/Down</kbd> " " {move || t::command_palette::keyboard_navigate_hint(locale.get())}</span>
                        <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Enter</kbd> " " {move || t::command_palette::keyboard_select_hint(locale.get())}</span>
                    </div>
                    <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Esc</kbd> " " {move || t::command_palette::keyboard_close_hint(locale.get())}</span>
                </div>
            }
            .into_any(),
        }}
    }
}
