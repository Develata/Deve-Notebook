//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 15_release#runtime-observability
//!
use crate::i18n::{Locale, t};
use deve_core::protocol::ServerErrorCode;
use leptos::prelude::*;

pub fn error_notice(
    error_code: ReadSignal<Option<ServerErrorCode>>,
    locale: RwSignal<Locale>,
    retry: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || error_code.get().is_some()>
            <div class="mx-2 mb-2 rounded border border-red-200 bg-red-50 px-2 py-2 text-xs text-red-700 flex items-center justify-between gap-2">
                <div class="min-w-0 truncate">
                    {move || {
                        let suffix = error_code
                            .get()
                            .map(|code| t::server_error::message(locale.get(), code))
                            .unwrap_or("");
                        format!("{}: {}", t::chat::send_failed(locale.get()), suffix)
                    }}
                </div>
                <button
                    class="h-11 min-w-11 px-3 rounded bg-panel border border-red-200 text-red-700 active:bg-red-100"
                    on:click=move |_| retry.run(())
                >
                    {move || t::chat::retry(locale.get())}
                </button>
            </div>
        </Show>
    }
}

pub fn loading_notice(loading: Signal<bool>, locale: RwSignal<Locale>) -> impl IntoView {
    view! {
        <Show when=move || loading.get()>
            <div class="px-3 pb-1 text-[11px] text-muted">
                {move || t::chat::loading(locale.get())}
            </div>
        </Show>
    }
}
