//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-responsive-layout
//!   - 15_release#runtime-observability
//!
use super::footer_playback::{PlaybackNarrow, PlaybackWide};
use super::footer_status::LoadStatus;
use crate::i18n::Locale;
use leptos::prelude::*;

#[component]
pub fn FooterDetails(
    load_state: ReadSignal<String>,
    load_progress: ReadSignal<(usize, usize)>,
    load_eta_ms: ReadSignal<u64>,
    is_narrow: ReadSignal<bool>,
    locale: RwSignal<Locale>,
    displayed_curr_ver: Signal<u64>,
    displayed_max_ver: Signal<u64>,
    on_to_start: Callback<()>,
    on_prev: Callback<()>,
    on_next: Callback<()>,
    on_to_end: Callback<()>,
    set_ver: WriteSignal<u64>,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2 overflow-x-auto pb-0.5 scrollbar-none">
            <Show when=move || load_state.get() != "ready">
                <div class="shrink-0 px-2 h-7 rounded-md bg-sidebar border border-default flex items-center">
                    <LoadStatus
                        load_state=load_state
                        load_progress=load_progress
                        load_eta_ms=load_eta_ms
                        is_narrow=is_narrow
                        locale=locale
                    />
                </div>
            </Show>
            <div class="shrink-0 text-[10px] text-muted font-mono px-1">
                {move || format!("v{}/{}", displayed_curr_ver.get(), displayed_max_ver.get())}
            </div>
        </div>

        <Show
            when=move || is_narrow.get()
            fallback=move || view! {
                <PlaybackWide
                    curr_ver=displayed_curr_ver
                    max_ver=displayed_max_ver
                    on_to_start=on_to_start
                    on_prev=on_prev
                    on_next=on_next
                    on_to_end=on_to_end
                    set_ver=set_ver
                    locale=locale
                />
            }
        >
            <PlaybackNarrow
                curr_ver=displayed_curr_ver
                max_ver=displayed_max_ver
                on_to_start=on_to_start
                on_prev=on_prev
                on_next=on_next
                on_to_end=on_to_end
                set_ver=set_ver
                locale=locale
            />
        </Show>
    }
}
