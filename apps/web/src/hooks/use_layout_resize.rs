use super::use_layout_storage::clamp;
use leptos::prelude::*;
use web_sys::PointerEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResizeTarget {
    Left,
    Right,
    OuterLeft,
    OuterRight,
}

#[derive(Clone, Copy)]
pub(crate) struct ResizeStateSignals {
    pub is_resizing: ReadSignal<bool>,
    pub active_pointer: ReadSignal<Option<i32>>,
    pub start_x: ReadSignal<i32>,
    pub start_width: ReadSignal<i32>,
    pub active_resize: ReadSignal<Option<ResizeTarget>>,
}

#[derive(Clone, Copy)]
pub(crate) struct ResizeOutputSignals {
    pub set_sidebar_width: WriteSignal<i32>,
    pub set_right_width: WriteSignal<i32>,
    pub set_outer_gutter: WriteSignal<i32>,
}

#[derive(Clone, Copy)]
pub(crate) struct ResizeBounds {
    pub sidebar: (i32, i32),
    pub right: (i32, i32),
    pub outer: (i32, i32),
}

pub(crate) fn start_resize_callback(
    set_is_resizing: WriteSignal<bool>,
    set_active_resize: WriteSignal<Option<ResizeTarget>>,
    set_start_x: WriteSignal<i32>,
    set_start_width: WriteSignal<i32>,
    set_active_pointer: WriteSignal<Option<i32>>,
    current_width: ReadSignal<i32>,
    target: ResizeTarget,
) -> Callback<PointerEvent> {
    Callback::new(move |ev: PointerEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
        set_active_resize.set(Some(target));
        set_start_x.set(ev.client_x());
        set_start_width.set(current_width.get_untracked());
        set_active_pointer.set(Some(ev.pointer_id()));
    })
}

pub(crate) fn stop_resize_callback(
    set_is_resizing: WriteSignal<bool>,
    set_active_resize: WriteSignal<Option<ResizeTarget>>,
    set_active_pointer: WriteSignal<Option<i32>>,
) -> Callback<()> {
    Callback::new(move |_| {
        set_is_resizing.set(false);
        set_active_resize.set(None);
        set_active_pointer.set(None);
    })
}

pub(crate) fn do_resize_callback(
    state: ResizeStateSignals,
    outputs: ResizeOutputSignals,
    bounds: ResizeBounds,
) -> Callback<PointerEvent> {
    Callback::new(move |ev: PointerEvent| {
        if !state.is_resizing.get_untracked() {
            return;
        }
        if let Some(active) = state.active_pointer.get_untracked()
            && ev.pointer_id() != active
        {
            return;
        }
        let delta = ev.client_x() - state.start_x.get_untracked();
        match state.active_resize.get_untracked() {
            Some(ResizeTarget::Left) => {
                let width = state.start_width.get_untracked() + delta;
                outputs
                    .set_sidebar_width
                    .set(clamp(width, bounds.sidebar.0, bounds.sidebar.1));
            }
            Some(ResizeTarget::Right) => {
                let width = state.start_width.get_untracked() - delta;
                outputs
                    .set_right_width
                    .set(clamp(width, bounds.right.0, bounds.right.1));
            }
            Some(ResizeTarget::OuterLeft) => {
                let width = state.start_width.get_untracked() + delta;
                outputs
                    .set_outer_gutter
                    .set(clamp(width, bounds.outer.0, bounds.outer.1));
            }
            Some(ResizeTarget::OuterRight) => {
                let width = state.start_width.get_untracked() - delta;
                outputs
                    .set_outer_gutter
                    .set(clamp(width, bounds.outer.0, bounds.outer.1));
            }
            None => {}
        }
    })
}
