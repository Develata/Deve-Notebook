use leptos::prelude::*;

#[component]
pub fn DesktopSyncBanner(sync_banner: Signal<Option<String>>) -> impl IntoView {
    view! {
        <Show when=move || sync_banner.get().is_some()>
            <div class="mx-4 mt-2 rounded-lg border border-amber-300 bg-amber-100 px-3 py-2 text-xs font-medium text-amber-950">
                {move || sync_banner.get().unwrap_or_default()}
            </div>
        </Show>
    }
}
