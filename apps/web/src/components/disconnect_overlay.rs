// apps/web/src/components/disconnect_overlay.rs
//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!   - 09_auth#unauthorized-disconnected-ui
//!
use crate::api::ConnectionStatus;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn DisconnectedOverlay(status: Signal<ConnectionStatus>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    view! {
        {move || {
            let status = status.get();
            if matches!(status, ConnectionStatus::Connected | ConnectionStatus::Unauthorized) {
                view! {}.into_any()
            } else {
                let current_locale = locale.get();
                let (title, body) = overlay_copy(current_locale, status);
                view! {
                    <div class="fixed inset-0 z-[var(--z-toast)] bg-panel/80 backdrop-blur-sm flex flex-col items-center justify-center">
                        <div class="bg-panel p-8 rounded-xl shadow-lg border border-default text-center">
                            <div class="text-4xl mb-4">"🔒"</div>
                            <h1 class="text-2xl font-bold text-primary mb-2">{title}</h1>
                            <p class="text-secondary mb-6">{body}</p>
                            <div class="w-full bg-active rounded-full h-2.5">
                              <div class="bg-accent h-2.5 rounded-full animate-pulse" style="width: 100%"></div>
                            </div>
                            <div class="mt-4 text-sm text-muted">
                                {format!("Status: {}", status)}
                            </div>
                        </div>
                    </div>
                }.into_any()
            }
        }}
    }
}

fn overlay_copy(locale: Locale, status: ConnectionStatus) -> (&'static str, &'static str) {
    match status {
        ConnectionStatus::NativeBootstrapInvalid => (
            t::common::native_bootstrap_invalid_title(locale),
            t::common::native_bootstrap_invalid_body(locale),
        ),
        ConnectionStatus::NativeSessionPending => (
            t::common::native_session_pending_title(locale),
            t::common::native_session_pending_body(locale),
        ),
        ConnectionStatus::NativeServiceOffline => (
            t::common::native_service_offline_title(locale),
            t::common::native_service_offline_body(locale),
        ),
        ConnectionStatus::NativeReprobeRequired => (
            t::common::native_reprobe_required_title(locale),
            t::common::native_reprobe_required_body(locale),
        ),
        ConnectionStatus::Disconnected
        | ConnectionStatus::Connecting
        | ConnectionStatus::Unauthorized
        | ConnectionStatus::Connected => (
            t::common::disconnected(locale),
            t::common::reconnecting(locale),
        ),
    }
}
