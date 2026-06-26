//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 18_release#runtime-observability
//!
use crate::editor::EditorStats;
use crate::hooks::use_core::LoadPhase;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn BottomBarStats(
    locale: RwSignal<Locale>,
    displayed_stats: Signal<EditorStats>,
    load_state: ReadSignal<LoadPhase>,
    load_progress: ReadSignal<(usize, usize)>,
    load_eta_ms: ReadSignal<u64>,
) -> impl IntoView {
    let load_text = Memo::new(move |_| {
        let state = load_state.get();
        if state.is_ready() {
            return None;
        }
        let (done, total) = load_progress.get();
        let eta_ms = load_eta_ms.get();
        Some(if total > 0 {
            t::bottom_bar::loading_progress(locale.get(), done, total, eta_ms)
        } else {
            t::bottom_bar::loading(locale.get()).to_string()
        })
    });

    view! {
        <div class="flex items-center gap-4 text-xs text-muted">
            <Show when=move || load_text.get().is_some()>
                <div class="text-[10px] text-muted font-mono">
                    {move || load_text.get().unwrap_or_default()}
                </div>
            </Show>
            <div class="flex gap-1">
                <span>{move || t::bottom_bar::words(locale.get())}</span>
                <span class="font-mono text-primary">{move || displayed_stats.get().words}</span>
            </div>
            <div class="flex gap-1">
                <span>{move || t::bottom_bar::lines(locale.get())}</span>
                <span class="font-mono text-primary">{move || displayed_stats.get().lines}</span>
            </div>
            <div class="flex gap-1">
                <span>{move || t::bottom_bar::chars(locale.get())}</span>
                <span class="font-mono text-primary">{move || displayed_stats.get().chars}</span>
            </div>
        </div>
    }
}
