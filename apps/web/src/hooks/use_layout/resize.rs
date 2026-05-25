//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use super::storage::clamp;
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

fn resized_width_for_target(
    target: ResizeTarget,
    start_width: i32,
    delta: i32,
    bounds: ResizeBounds,
) -> i32 {
    match target {
        ResizeTarget::Left => clamp(start_width + delta, bounds.sidebar.0, bounds.sidebar.1),
        ResizeTarget::Right => clamp(start_width - delta, bounds.right.0, bounds.right.1),
        ResizeTarget::OuterLeft => clamp(start_width + delta, bounds.outer.0, bounds.outer.1),
        ResizeTarget::OuterRight => clamp(start_width - delta, bounds.outer.0, bounds.outer.1),
    }
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
                let width = resized_width_for_target(
                    ResizeTarget::Left,
                    state.start_width.get_untracked(),
                    delta,
                    bounds,
                );
                outputs.set_sidebar_width.set(width);
            }
            Some(ResizeTarget::Right) => {
                let width = resized_width_for_target(
                    ResizeTarget::Right,
                    state.start_width.get_untracked(),
                    delta,
                    bounds,
                );
                outputs.set_right_width.set(width);
            }
            Some(ResizeTarget::OuterLeft) => {
                let width = resized_width_for_target(
                    ResizeTarget::OuterLeft,
                    state.start_width.get_untracked(),
                    delta,
                    bounds,
                );
                outputs.set_outer_gutter.set(width);
            }
            Some(ResizeTarget::OuterRight) => {
                let width = resized_width_for_target(
                    ResizeTarget::OuterRight,
                    state.start_width.get_untracked(),
                    delta,
                    bounds,
                );
                outputs.set_outer_gutter.set(width);
            }
            None => {}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desktop_bounds() -> ResizeBounds {
        ResizeBounds {
            sidebar: (180, 500),
            right: (240, 520),
            outer: (0, 120),
        }
    }

    #[test]
    fn desktop_layout_resize_sidebar_clamps_to_bounds() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_width_for_target(ResizeTarget::Left, 260, 40, bounds),
            300
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::Left, 260, -200, bounds),
            180
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::Left, 260, 400, bounds),
            500
        );
    }

    #[test]
    fn desktop_layout_resize_right_panel_uses_inverse_delta_and_clamps() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_width_for_target(ResizeTarget::Right, 320, -60, bounds),
            380
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::Right, 320, 200, bounds),
            240
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::Right, 320, -300, bounds),
            520
        );
    }

    #[test]
    fn desktop_layout_resize_outer_gutter_uses_side_direction_and_clamps() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_width_for_target(ResizeTarget::OuterLeft, 48, 30, bounds),
            78
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::OuterLeft, 48, -80, bounds),
            0
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::OuterLeft, 48, 200, bounds),
            120
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::OuterRight, 48, -30, bounds),
            78
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::OuterRight, 48, 80, bounds),
            0
        );
        assert_eq!(
            resized_width_for_target(ResizeTarget::OuterRight, 48, -200, bounds),
            120
        );
    }
}
