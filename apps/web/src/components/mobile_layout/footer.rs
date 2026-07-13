// apps/web/src/components/mobile_layout/footer.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 18_release#runtime-observability
//!
//! # Mobile Footer
//!
//! Entry point. Delegates status/load to `footer_status`,
//! playback controls to `footer_playback`.

use super::footer_details::FooterDetails;
use super::footer_summary::FooterSummaryRow;
use crate::editor::EditorStats;
use crate::hooks::use_core::EditorContext;
use crate::i18n::Locale;
use crate::runtime::{document_client::DocumentClient, rendering_client::RenderingClient};
use leptos::prelude::*;
use web_sys::UiEvent;

use super::footer_read::read_footer_signal;

pub(super) fn bottom_bar_state_attrs(expanded: bool) -> (&'static str, &'static str) {
    if expanded {
        ("expanded", "multi")
    } else {
        ("collapsed", "single")
    }
}

pub(super) fn bottom_bar_after_outside_click(_expanded: bool) -> bool {
    false
}

#[component]
pub fn MobileFooter(pending_ack_count: Memo<usize>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let document = expect_context::<DocumentClient>();
    let editor = expect_context::<EditorContext>();
    let rendering = expect_context::<RenderingClient>();
    let max_ver = editor.doc_version;
    let curr_ver = editor.playback_version;
    let set_ver = editor.set_playback_version;
    let current_doc = document.current_doc;
    let stats = rendering.stats;
    let (is_narrow, set_is_narrow) = signal(false);
    let (expanded, set_expanded) = signal(false);
    let displayed_stats = Signal::derive(move || {
        if current_doc.get().is_some() {
            stats.get()
        } else {
            EditorStats::default()
        }
    });
    let displayed_max_ver = Signal::derive(move || {
        if current_doc.get().is_some() {
            max_ver.get()
        } else {
            0
        }
    });
    let displayed_curr_ver = Signal::derive(move || {
        if current_doc.get().is_some() {
            curr_ver.get()
        } else {
            0
        }
    });

    let update_narrow = move || {
        let width = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(390.0);
        set_is_narrow.set(width <= 360.0);
    };
    update_narrow();
    window_event_listener(leptos::ev::resize, move |_ev: UiEvent| update_narrow());

    let on_to_start = Callback::new(move |_| set_ver.set(0));
    let on_prev = Callback::new(move |_| {
        let next = curr_ver.get_untracked().saturating_sub(1);
        set_ver.set(next);
    });
    let on_next = Callback::new(move |_| {
        let cur = curr_ver.get_untracked();
        let max = max_ver.get_untracked();
        set_ver.set((cur + 1).min(max));
    });
    let on_to_end = Callback::new(move |_| set_ver.set(max_ver.get_untracked()));

    let footer_class = move || {
        if read_footer_signal(expanded, false) {
            "relative z-[calc(var(--z-overlay)_+_1)] bg-panel border-t border-default px-2 py-1.5 flex flex-col gap-1.5"
        } else {
            "relative z-[var(--z-panels)] bg-panel border-t border-default px-2 py-1.5 flex flex-col gap-1.5"
        }
    };

    view! {
        <Show when=move || read_footer_signal(expanded, false)>
            <div
                data-deve-mobile-bottom-bar-dismiss="outside_bottom_bar"
                class="fixed inset-0 z-[var(--z-overlay)]"
                on:click=move |_| set_expanded.set(bottom_bar_after_outside_click(expanded.get_untracked()))
            ></div>
        </Show>

        <footer
            data-deve-mobile-bottom-bar=move || bottom_bar_state_attrs(read_footer_signal(expanded, false)).0
            data-deve-mobile-bottom-bar-lines=move || bottom_bar_state_attrs(read_footer_signal(expanded, false)).1
            class=footer_class
            style="padding-bottom: env(safe-area-inset-bottom);"
        >
            <FooterSummaryRow
                locale=locale
                is_narrow=is_narrow
                expanded=expanded
                set_expanded=set_expanded
                displayed_stats=displayed_stats
                pending_ack_count=pending_ack_count
            />

            <Show when=move || read_footer_signal(expanded, false)>
                <FooterDetails
                    load_state=rendering.load_state
                    load_progress=rendering.load_progress
                    load_eta_ms=rendering.load_eta_ms
                    is_narrow=is_narrow
                    locale=locale
                    displayed_curr_ver=displayed_curr_ver
                    displayed_max_ver=displayed_max_ver
                    on_to_start=on_to_start
                    on_prev=on_prev
                    on_next=on_next
                    on_to_end=on_to_end
                    set_ver=set_ver
                />
            </Show>
        </footer>
    }
}

#[cfg(test)]
mod tests {
    use super::{bottom_bar_after_outside_click, bottom_bar_state_attrs};

    #[test]
    fn mobile_bottom_bar_collapsed_state_is_single_line() {
        assert_eq!(bottom_bar_state_attrs(false), ("collapsed", "single"));
        assert_eq!(bottom_bar_state_attrs(true), ("expanded", "multi"));
    }

    #[test]
    fn mobile_bottom_bar_expand_outside_click_collapses() {
        assert!(!bottom_bar_after_outside_click(true));
        assert!(!bottom_bar_after_outside_click(false));
    }
}
