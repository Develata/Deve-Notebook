use crate::i18n::{Locale, common};
use leptos::prelude::*;

fn hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unable to reach the auth service. We'll retry automatically.",
        Locale::Zh => "当前无法连接认证服务，系统会自动重试。",
    }
}

#[component]
pub fn AuthUnavailablePage() -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    view! {
        <div class="fixed inset-0 bg-bg flex items-center justify-center z-50">
            <div class="w-full max-w-sm p-8 bg-bg-panel rounded-lg shadow-lg border border-border text-center">
                <div class="text-4xl mb-4">"🔒"</div>
                <h1 class="text-2xl font-bold text-primary mb-2">
                    {move || common::disconnected(locale.get())}
                </h1>
                <p class="text-sm text-muted mb-2">
                    {move || common::reconnecting(locale.get())}
                </p>
                <p class="text-xs text-muted">{move || hint(locale.get())}</p>
            </div>
        </div>
    }
}
