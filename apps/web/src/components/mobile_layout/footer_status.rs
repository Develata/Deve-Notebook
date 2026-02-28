// apps/web/src/components/mobile_layout/footer_status.rs
//! # Mobile Footer — Status & Load Indicators

use crate::api::ConnectionStatus;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

/// Connection status indicator (green/yellow/red dot + text).
#[component]
pub fn StatusView(status: ReadSignal<ConnectionStatus>, locale: RwSignal<Locale>) -> impl IntoView {
    let (color, text) = match status.get() {
        ConnectionStatus::Connected => ("bg-green-500", t::bottom_bar::ready(locale.get())),
        ConnectionStatus::Connecting => ("bg-yellow-500", t::bottom_bar::syncing(locale.get())),
        ConnectionStatus::Disconnected => ("bg-red-500", t::bottom_bar::offline(locale.get())),
    };
    view! {
        <div class="flex items-center gap-1.5">
            <div class={format!("w-2 h-2 rounded-full {}", color)}></div>
            <span class="text-[11px] text-secondary font-medium">{text}</span>
        </div>
    }
}

/// Loading progress bar (hidden when `load_state == "ready"`).
#[component]
pub fn LoadStatus(
    load_state: ReadSignal<String>,
    load_progress: ReadSignal<(usize, usize)>,
    load_eta_ms: ReadSignal<u64>,
    is_narrow: ReadSignal<bool>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    move || {
        if load_state.get() == "ready" {
            return view! {}.into_any();
        }
        let (done, total) = load_progress.get();
        let eta_ms = load_eta_ms.get();
        let text = if total > 0 {
            if eta_ms > 0 && !is_narrow.get() {
                format!(
                    "{} {}/{} (~{}ms)",
                    t::bottom_bar::loading(locale.get()),
                    done,
                    total,
                    eta_ms,
                )
            } else {
                format!("L {}/{}", done, total)
            }
        } else {
            t::bottom_bar::loading(locale.get()).to_string()
        };
        view! { <div class="text-[10px] text-muted font-mono">{text}</div> }.into_any()
    }
}
