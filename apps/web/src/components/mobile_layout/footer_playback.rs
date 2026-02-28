// apps/web/src/components/mobile_layout/footer_playback.rs
//! # Mobile Footer — Time-Travel Playback Controls

use crate::i18n::{Locale, t};
use leptos::prelude::*;

/// Wide-screen playback: horizontal button row + range slider.
#[component]
pub fn PlaybackWide(
    curr_ver: ReadSignal<u64>,
    max_ver: ReadSignal<u64>,
    on_to_start: Callback<()>,
    on_prev: Callback<()>,
    on_next: Callback<()>,
    on_to_end: Callback<()>,
    set_ver: WriteSignal<u64>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-1.5 px-1 py-1 rounded-lg bg-sidebar border border-default">
            <button class="h-7 min-w-7 px-1 rounded border border-default bg-panel text-secondary active:bg-hover" title=move || t::bottom_bar::first(locale.get()) on:click=move |_| on_to_start.run(())>
                "«"
            </button>
            <button class="h-7 min-w-7 px-1 rounded border border-default bg-panel text-secondary active:bg-hover" title=move || t::bottom_bar::prev(locale.get()) on:click=move |_| on_prev.run(())>
                "‹"
            </button>
            <input
                name="mobile-time-travel"
                type="range"
                min="0"
                max=move || max_ver.get().to_string()
                value=move || curr_ver.get().to_string()
                on:input=move |ev| {
                    let val = event_target_value(&ev).parse::<u64>().unwrap_or(0);
                    set_ver.set(val);
                }
                class="flex-1 min-w-16 h-1 bg-active rounded-lg appearance-none cursor-pointer accent-accent"
                title=move || t::bottom_bar::time_travel(locale.get())
            />
            <button class="h-7 min-w-7 px-1 rounded border border-default bg-panel text-secondary active:bg-hover" title=move || t::bottom_bar::next(locale.get()) on:click=move |_| on_next.run(())>
                "›"
            </button>
            <button class="h-7 min-w-7 px-1 rounded border border-default bg-panel text-secondary active:bg-hover" title=move || t::bottom_bar::latest(locale.get()) on:click=move |_| on_to_end.run(())>
                "»"
            </button>
        </div>
    }
}

/// Narrow-screen playback: stacked buttons + version label + range slider.
#[component]
pub fn PlaybackNarrow(
    curr_ver: ReadSignal<u64>,
    max_ver: ReadSignal<u64>,
    on_to_start: Callback<()>,
    on_prev: Callback<()>,
    on_next: Callback<()>,
    on_to_end: Callback<()>,
    set_ver: WriteSignal<u64>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        <div class="rounded-lg bg-sidebar border border-default px-1 py-1 flex flex-col gap-1">
            <div class="flex items-center justify-between gap-1">
                <button class="h-6 min-w-6 px-1 rounded border border-default bg-panel text-secondary active:bg-hover text-[10px]" title=move || t::bottom_bar::first(locale.get()) on:click=move |_| on_to_start.run(())>
                    "«"
                </button>
                <button class="h-6 min-w-6 px-1 rounded border border-default bg-panel text-secondary active:bg-hover text-[10px]" title=move || t::bottom_bar::prev(locale.get()) on:click=move |_| on_prev.run(())>
                    "‹"
                </button>
                <span class="text-[9px] text-muted font-mono px-0.5 min-w-12 text-center">
                    {move || format!("v{}/{}", curr_ver.get(), max_ver.get())}
                </span>
                <button class="h-6 min-w-6 px-1 rounded border border-default bg-panel text-secondary active:bg-hover text-[10px]" title=move || t::bottom_bar::next(locale.get()) on:click=move |_| on_next.run(())>
                    "›"
                </button>
                <button class="h-6 min-w-6 px-1 rounded border border-default bg-panel text-secondary active:bg-hover text-[10px]" title=move || t::bottom_bar::latest(locale.get()) on:click=move |_| on_to_end.run(())>
                    "»"
                </button>
            </div>
            <input
                name="mobile-time-travel-narrow"
                type="range"
                min="0"
                max=move || max_ver.get().to_string()
                value=move || curr_ver.get().to_string()
                on:input=move |ev| {
                    let val = event_target_value(&ev).parse::<u64>().unwrap_or(0);
                    set_ver.set(val);
                }
                class="w-full h-1 bg-active rounded-lg appearance-none cursor-pointer accent-accent"
                title=move || t::bottom_bar::time_travel(locale.get())
            />
        </div>
    }
}
