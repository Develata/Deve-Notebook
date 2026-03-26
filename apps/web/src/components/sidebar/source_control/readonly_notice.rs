use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn ReadonlyNotice() -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <div class="px-4 py-3 text-sm">
            <p class="text-primary">{move || t::source_control::remote_branch_readonly(locale.get())}</p>
            <p class="mt-1 text-xs text-muted">
                {move || t::source_control::remote_branch_readonly_hint(locale.get())}
            </p>
        </div>
    }
}
