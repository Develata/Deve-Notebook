use super::anchor::{ScrollAnchor, active_container, capture_anchor, restore_anchor};
use super::navigation::should_ignore_shortcut;
use super::state::ComputePhase;
use leptos::html;
use leptos::prelude::*;

pub fn setup_anchor_effects(
    force_unified: bool,
    compute_state: ReadSignal<ComputePhase>,
    unified_ref: NodeRef<html::Div>,
    left_ref: NodeRef<html::Div>,
) {
    let (anchor_state, set_anchor_state) = signal(None::<ScrollAnchor>);

    Effect::new(move |_| {
        if compute_state.get() != ComputePhase::Computing {
            return;
        }
        if let Some(container) = active_container(force_unified, unified_ref, left_ref) {
            set_anchor_state.set(capture_anchor(&container));
        }
    });

    Effect::new(move |_| {
        if compute_state.get() != ComputePhase::Ready {
            return;
        }
        let Some(anchor) = anchor_state.get() else {
            return;
        };
        if let Some(container) = active_container(force_unified, unified_ref, left_ref) {
            let _ = restore_anchor(&container, &anchor);
        }
    });
}

pub fn setup_shortcuts(
    on_close: Callback<()>,
    on_prev_hunk: Callback<()>,
    on_next_hunk: Callback<()>,
) {
    let _esc_listener =
        window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
            if should_ignore_shortcut(&ev) {
                return;
            }
            if ev.key() == "Escape" {
                ev.prevent_default();
                on_close.run(());
                return;
            }
            let key = ev.key();
            if key == "]"
                || (ev.alt_key() && key == "ArrowDown")
                || (key == "F7" && !ev.shift_key())
            {
                ev.prevent_default();
                on_next_hunk.run(());
                return;
            }
            if key == "[" || (ev.alt_key() && key == "ArrowUp") || (key == "F7" && ev.shift_key()) {
                ev.prevent_default();
                on_prev_hunk.run(());
            }
        });
}
