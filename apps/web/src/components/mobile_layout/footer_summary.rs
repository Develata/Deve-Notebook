use super::footer_status::StatusView;
use crate::components::branch_switcher::BranchSwitcher;
use crate::components::icons::{ChevronDown, ChevronUp};
use crate::editor::EditorStats;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn FooterSummaryRow(
    core: CoreState,
    locale: RwSignal<Locale>,
    is_narrow: ReadSignal<bool>,
    expanded: ReadSignal<bool>,
    set_expanded: WriteSignal<bool>,
    displayed_stats: Signal<EditorStats>,
) -> impl IntoView {
    let stat_label = move |compact: &'static str, full: fn(Locale) -> &'static str| {
        let locale = locale;
        move || {
            if is_narrow.get() {
                compact.to_string()
            } else {
                full(locale.get()).to_string()
            }
        }
    };

    view! {
        <div class="flex items-center gap-1.5">
            <div class="flex-1 min-w-0 flex items-center gap-1 whitespace-nowrap overflow-hidden">
                <div class="shrink-0"><BranchSwitcher compact=true /></div>
                <div class="shrink-0 px-1.5 h-6 rounded-md bg-sidebar border border-default flex items-center">
                    {move || view! { <StatusView core=core.clone() locale=locale /> }}
                </div>
                <div class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                    <span>{stat_label("W", t::bottom_bar::words)}</span>
                    <span class="font-mono text-primary">{move || displayed_stats.get().words}</span>
                </div>
                <div class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                    <span>{stat_label("L", t::bottom_bar::lines)}</span>
                    <span class="font-mono text-primary">{move || displayed_stats.get().lines}</span>
                </div>
                <div class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                    <span>{stat_label("Ch", t::bottom_bar::chars)}</span>
                    <span class="font-mono text-primary">{move || displayed_stats.get().chars}</span>
                </div>
            </div>

            <button
                class="h-11 min-w-11 p-1.5 rounded-md active:bg-hover flex items-center justify-center"
                title=move || t::bottom_bar::toggle_status_details(locale.get())
                aria-label=move || t::bottom_bar::toggle_status_details(locale.get())
                on:click=move |_| set_expanded.update(|v| *v = !*v)
            >
                {move || if expanded.get() {
                    view! {
                        <span class="h-8 w-8 rounded-md border border-default bg-panel text-secondary flex items-center justify-center">
                            <ChevronDown class="w-4 h-4"/>
                        </span>
                    }.into_any()
                } else {
                    view! {
                        <span class="h-8 w-8 rounded-md border border-default bg-panel text-secondary flex items-center justify-center">
                            <ChevronUp class="w-4 h-4"/>
                        </span>
                    }.into_any()
                }}
            </button>
        </div>
    }
}
