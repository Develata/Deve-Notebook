//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 15_release#runtime-observability
//!
use crate::i18n::{Locale, t};
use leptos::ev::Event;
use leptos::prelude::*;

#[component]
pub fn BottomBarTimeTravel(
    locale: RwSignal<Locale>,
    displayed_curr_ver: Signal<u64>,
    displayed_max_ver: Signal<u64>,
    set_ver: WriteSignal<u64>,
) -> impl IntoView {
    let update_version = move |ev: Event| {
        let val = event_target_value(&ev).parse::<u64>().unwrap_or(0);
        set_ver.set(val);
    };

    view! {
        <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 flex items-center gap-2">
            <span class="text-[10px] text-muted font-mono">
                {move || format!("v{}/{}", displayed_curr_ver.get(), displayed_max_ver.get())}
            </span>
            <input
                name="time-travel"
                type="range"
                min="0"
                max=move || displayed_max_ver.get().to_string()
                value=move || displayed_curr_ver.get().to_string()
                on:input=update_version
                class="w-32 h-1 bg-active rounded-lg appearance-none cursor-pointer accent-accent"
                title=move || t::bottom_bar::time_travel(locale.get())
            />
        </div>
    }
}
