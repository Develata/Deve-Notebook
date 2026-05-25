//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 18_release#runtime-observability
//!
use crate::i18n::{Locale, t};
use deve_core::protocol::ServerErrorCode;
use leptos::prelude::*;

pub(crate) fn chat_error_banner_marker(has_error: bool) -> Option<&'static str> {
    has_error.then_some("visible")
}

pub(crate) fn chat_retry_button_marker(has_error: bool) -> Option<&'static str> {
    has_error.then_some("chat_retry_button")
}

pub(crate) fn chat_retry_button_class() -> &'static str {
    "h-11 min-w-[44px] px-3 rounded bg-panel border border-red-200 text-red-700 active:bg-red-100"
}

pub fn error_notice(
    error_code: ReadSignal<Option<ServerErrorCode>>,
    locale: RwSignal<Locale>,
    retry: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || error_code.get().is_some()>
            <div
                data-deve-chat-error-banner=move || chat_error_banner_marker(error_code.get().is_some())
                class="mx-2 mb-2 rounded border border-red-200 bg-red-50 px-2 py-2 text-xs text-red-700 flex items-center justify-between gap-2"
            >
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
                    data-deve-chat-action=move || chat_retry_button_marker(error_code.get().is_some())
                    class=chat_retry_button_class()
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

#[cfg(test)]
mod tests {
    use super::{chat_error_banner_marker, chat_retry_button_class, chat_retry_button_marker};

    #[test]
    fn mobile_chat_error_banner_marker_is_visible_only_on_error() {
        assert_eq!(chat_error_banner_marker(true), Some("visible"));
        assert_eq!(chat_error_banner_marker(false), None);
    }

    #[test]
    fn mobile_chat_error_retry_button_marker_is_stable() {
        assert_eq!(chat_retry_button_marker(true), Some("chat_retry_button"));
        assert_eq!(chat_retry_button_marker(false), None);
    }

    #[test]
    fn mobile_chat_error_retry_button_is_at_least_44px() {
        let class = chat_retry_button_class();

        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }
}
