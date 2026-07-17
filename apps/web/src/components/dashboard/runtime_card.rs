//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 18_release#runtime-observability
//!
//! Runtime release shape card.

use crate::api::{WatcherHealthSnapshot, WatcherHealthStatus};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn RuntimeCard(
    runtime_summary: ReadSignal<String>,
    watcher_health: ReadSignal<WatcherHealthSnapshot>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let summary = Signal::derive(move || {
        let value = runtime_summary.get();
        if value.is_empty() {
            t::dashboard::runtime_waiting(locale.get()).to_string()
        } else {
            value
        }
    });
    let watcher_status = Signal::derive(move || {
        let locale = locale.get();
        match watcher_health.get().status {
            WatcherHealthStatus::Healthy => t::workspace_ingestion::health_healthy(locale),
            WatcherHealthStatus::Transitioning => {
                t::workspace_ingestion::health_transitioning(locale)
            }
            WatcherHealthStatus::Degraded => t::workspace_ingestion::health_degraded(locale),
            WatcherHealthStatus::Unknown => t::workspace_ingestion::health_unknown(locale),
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
                <div
                    class="flex items-start justify-between gap-3"
                    data-deve-watcher-health-status=move || watcher_health.get().status.as_str()
                    data-deve-watcher-health-expected=move || watcher_health.get().expected.to_string()
                    data-deve-watcher-health-running=move || watcher_health.get().running.to_string()
                    data-deve-watcher-health-unavailable=move || watcher_health.get().unavailable.to_string()
                >
                    <span class="text-xs text-muted shrink-0">{move || watcher_status.get()}</span>
                    <span class="text-right text-xs font-mono text-primary">
                        {move || {
                            let health = watcher_health.get();
                            t::workspace_ingestion::health_counts(
                                locale.get(),
                                health.running,
                                health.expected,
                                health.unavailable,
                            )
                        }}
                    </span>
                </div>
            </div>
        </div>
    }
}
