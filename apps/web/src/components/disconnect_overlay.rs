// apps/web/src/components/disconnect_overlay.rs
use crate::api::ConnectionStatus;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

fn dev_non_blocking_overlay() -> bool {
    cfg!(debug_assertions)
}

#[component]
pub fn DisconnectedOverlay(status: Signal<ConnectionStatus>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    view! {
        {move || {
            let status = status.get();
            if matches!(status, ConnectionStatus::Connected | ConnectionStatus::Unauthorized) {
                view! {}.into_any()
            } else if dev_non_blocking_overlay() {
                view! {
                    <div class="fixed right-4 top-4 z-[9999] pointer-events-none">
                        <div class="rounded-lg border border-default bg-panel/95 px-4 py-3 shadow-lg backdrop-blur-sm">
                            <div class="text-xs font-semibold uppercase tracking-[0.2em] text-muted">
                                "Dev WS Status"
                            </div>
                            <div class="mt-1 text-sm text-primary">
                                {format!("Status: {}", status)}
                            </div>
                            <div class="mt-1 text-xs text-secondary">
                                "Overlay downgraded in debug build so the underlying UI stays interactive."
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="fixed inset-0 z-[9999] bg-panel/80 backdrop-blur-sm flex flex-col items-center justify-center">
                        <div class="bg-panel p-8 rounded-xl shadow-lg border border-default text-center">
                            <div class="text-4xl mb-4">"🔒"</div>
                            <h1 class="text-2xl font-bold text-primary mb-2">{move || t::common::disconnected(locale.get())}</h1>
                            <p class="text-secondary mb-6">{move || t::common::reconnecting(locale.get())}</p>
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
