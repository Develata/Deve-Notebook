//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 18_release#runtime-observability
//!
use super::footer_playback::{PlaybackNarrow, PlaybackWide};
use super::footer_read::{read_footer_signal, read_footer_value};
use super::footer_status::LoadStatus;
use crate::i18n::Locale;
use crate::runtime::domain::LoadPhase;
use leptos::prelude::*;

#[component]
pub fn FooterDetails(
    load_state: ReadSignal<LoadPhase>,
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
        <div
            id="deve-mobile-bottom-bar-details"
            data-deve-mobile-bottom-bar-details="expanded"
            class="flex items-center gap-2 overflow-x-auto pb-0.5 scrollbar-none"
        >
            <Show when=move || !read_footer_signal(load_state, LoadPhase::Ready).is_ready()>
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
                {move || {
                    format!(
                        "v{}/{}",
                        read_footer_value(displayed_curr_ver, 0),
                        read_footer_value(displayed_max_ver, 0),
                    )
                }}
            </div>
        </div>

        <Show
            when=move || read_footer_signal(is_narrow, false)
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
