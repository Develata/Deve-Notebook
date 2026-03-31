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
    is_resizing: ReadSignal<bool>,
    active_pointer: ReadSignal<Option<i32>>,
    start_x: ReadSignal<i32>,
    start_width: ReadSignal<i32>,
    active_resize: ReadSignal<Option<ResizeTarget>>,
    set_sidebar_width: WriteSignal<i32>,
    set_right_width: WriteSignal<i32>,
    set_outer_gutter: WriteSignal<i32>,
    sidebar_bounds: (i32, i32),
    right_bounds: (i32, i32),
    outer_bounds: (i32, i32),
) -> Callback<PointerEvent> {
    Callback::new(move |ev: PointerEvent| {
        if !is_resizing.get_untracked() {
            return;
        }
        if let Some(active) = active_pointer.get_untracked()
            && ev.pointer_id() != active
        {
            return;
        }
        let delta = ev.client_x() - start_x.get_untracked();
        match active_resize.get_untracked() {
            Some(ResizeTarget::Left) => {
                let width = start_width.get_untracked() + delta;
                set_sidebar_width.set(clamp(width, sidebar_bounds.0, sidebar_bounds.1));
            }
            Some(ResizeTarget::Right) => {
                let width = start_width.get_untracked() - delta;
                set_right_width.set(clamp(width, right_bounds.0, right_bounds.1));
            }
            Some(ResizeTarget::OuterLeft) => {
                let width = start_width.get_untracked() + delta;
                set_outer_gutter.set(clamp(width, outer_bounds.0, outer_bounds.1));
            }
            Some(ResizeTarget::OuterRight) => {
                let width = start_width.get_untracked() - delta;
                set_outer_gutter.set(clamp(width, outer_bounds.0, outer_bounds.1));
            }
            None => {}
        }
    })
}
