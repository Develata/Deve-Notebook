//! plan_ref:
//!   - 18_release#runtime-observability
//!
//! Runtime release shape card.

use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn RuntimeCard(runtime_summary: ReadSignal<String>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let summary = Signal::derive(move || {
        let value = runtime_summary.get();
        if value.is_empty() {
            t::dashboard::runtime_waiting(locale.get()).to_string()
        } else {
            value
        }
    });

    view! {
        <div class="bg-panel rounded-lg border border-default p-4">
            <h3 class="text-sm font-semibold text-secondary mb-3">{move || t::dashboard::runtime_info(locale.get())}</h3>
            <div class="space-y-2">
                <div class="flex items-start justify-between gap-3">
                    <span class="text-xs text-muted shrink-0">{move || t::dashboard::runtime_shape(locale.get())}</span>
                    <span class="text-right text-xs font-mono leading-relaxed text-primary break-words">
                        {move || summary.get()}
                    </span>
                </div>
            </div>
        </div>
    }
}
