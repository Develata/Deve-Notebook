//! plan_ref:
//!   - 09_auth#unauthorized-disconnected-ui
//!

use crate::i18n::{Locale, common, login as login_i18n};
use leptos::prelude::*;

#[component]
pub fn AuthUnavailablePage() -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    view! {
        <div class="fixed inset-0 bg-bg flex items-center justify-center z-[var(--z-modal)]">
            <div class="w-full max-w-sm p-8 bg-bg-panel rounded-lg shadow-lg border border-border text-center">
                <div class="text-4xl mb-4">"🔒"</div>
                <h1 class="text-2xl font-bold text-primary mb-2">
                    {move || common::disconnected(locale.get())}
                </h1>
                <p class="text-sm text-muted mb-2">
                    {move || common::reconnecting(locale.get())}
                </p>
                <p class="text-xs text-muted">
                    {move || login_i18n::auth_unavailable_hint(locale.get())}
                </p>
            </div>
        </div>
    }
}
