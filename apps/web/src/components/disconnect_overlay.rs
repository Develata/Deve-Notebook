// apps/web/src/components/disconnect_overlay.rs
use crate::api::ConnectionStatus;
use leptos::prelude::*;

#[component]
pub fn DisconnectedOverlay(status: Signal<ConnectionStatus>) -> impl IntoView {
    view! {
        {move || {
            let status = status.get();
            if status != ConnectionStatus::Connected {
                view! {
                    <div class="fixed inset-0 z-[9999] bg-panel/80 backdrop-blur-sm flex flex-col items-center justify-center">
                        <div class="bg-panel p-8 rounded-xl shadow-lg border border-default text-center">
                            <div class="text-4xl mb-4">"🔒"</div>
                            <h1 class="text-2xl font-bold text-primary mb-2">"Disconnected"</h1>
                            <p class="text-secondary mb-6">"Reconnecting to server... please wait."</p>
                            <div class="w-full bg-active rounded-full h-2.5">
                              <div class="bg-accent h-2.5 rounded-full animate-pulse" style="width: 100%"></div>
                            </div>
                            <div class="mt-4 text-sm text-muted">
                                {format!("Status: {}", status)}
                            </div>
                        </div>
                    </div>
                }
                .into_any()
            } else {
                view! {}.into_any()
            }
        }}
    }
}
