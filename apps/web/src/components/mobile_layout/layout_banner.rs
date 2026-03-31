use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

#[component]
pub fn MobileSyncBanner(banner_toggle: CoreState, banner_text: CoreState) -> impl IntoView {
    view! {
        <Show when=move || banner_toggle.sync_banner.get().is_some()>
            <div class="mx-3 mt-2 rounded-lg border border-amber-300 bg-amber-100 px-3 py-2 text-[11px] font-medium text-amber-950">
                {move || banner_text.sync_banner.get().unwrap_or_default()}
            </div>
        </Show>
    }
}
