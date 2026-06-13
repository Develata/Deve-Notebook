//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use super::storage::clamp;
use leptos::prelude::*;
use web_sys::PointerEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResizeTarget {
    LeftDivider,
    RightDivider,
    OuterLeft,
    OuterRight,
}

#[derive(Clone, Copy)]
pub(crate) struct ResizeStateSignals {
    pub is_resizing: ReadSignal<bool>,
    pub active_pointer: ReadSignal<Option<i32>>,
    pub start_x: ReadSignal<i32>,
    pub start_left_width: ReadSignal<i32>,
    pub start_right_width: ReadSignal<i32>,
    pub start_effective_left_width: ReadSignal<i32>,
    pub start_effective_right_width: ReadSignal<i32>,
    pub start_outer_gutter: ReadSignal<i32>,
    pub active_resize: ReadSignal<Option<ResizeTarget>>,
    pub available_panel_width: Signal<i32>,
}

#[derive(Clone, Copy)]
pub(crate) struct ResizeOutputSignals {
    pub set_left_width: WriteSignal<i32>,
    pub set_right_width: WriteSignal<i32>,
    pub set_outer_gutter: WriteSignal<i32>,
}

#[derive(Clone, Copy)]
pub(crate) struct ResizeBounds {
    pub panel_width: (i32, i32),
    pub outer: (i32, i32),
}

#[cfg(test)]
fn panel_center_width(left_width: i32, right_width: i32, available_width: i32) -> i32 {
    (available_width - left_width - right_width).max(0)
}

#[derive(Debug, PartialEq, Eq)]
struct ResizeResult {
    left_width: i32,
    right_width: i32,
    outer_gutter: i32,
}

#[cfg(test)]
fn resized_values_for_target(
    target: ResizeTarget,
    start_left_width: i32,
    start_right_width: i32,
    start_outer_gutter: i32,
    delta: i32,
    available_width: i32,
    bounds: ResizeBounds,
) -> ResizeResult {
    resized_values_for_target_with_constraints(
        target,
        start_left_width,
        start_right_width,
        start_left_width,
        start_right_width,
        start_outer_gutter,
        delta,
        available_width,
        bounds,
    )
}

fn resized_values_for_target_with_constraints(
    target: ResizeTarget,
    start_left_width: i32,
    start_right_width: i32,
    start_effective_left_width: i32,
    start_effective_right_width: i32,
    start_outer_gutter: i32,
    delta: i32,
    available_width: i32,
    bounds: ResizeBounds,
) -> ResizeResult {
    let available_width = clamp(available_width, bounds.panel_width.0, bounds.panel_width.1);

    match target {
        ResizeTarget::LeftDivider => {
            let max_left = available_width
                .saturating_sub(start_effective_right_width)
                .max(0);
            ResizeResult {
                left_width: clamp(start_left_width.saturating_add(delta), 0, max_left),
                right_width: start_right_width,
                outer_gutter: start_outer_gutter,
            }
        }
        ResizeTarget::RightDivider => {
            let max_right = available_width
                .saturating_sub(start_effective_left_width)
                .max(0);
            ResizeResult {
                left_width: start_left_width,
                right_width: clamp(start_right_width.saturating_sub(delta), 0, max_right),
                outer_gutter: start_outer_gutter,
            }
        }
        ResizeTarget::OuterLeft => ResizeResult {
            left_width: start_left_width,
            right_width: start_right_width,
            outer_gutter: clamp(
                start_outer_gutter.saturating_add(delta),
                bounds.outer.0,
                bounds.outer.1,
            ),
        },
        ResizeTarget::OuterRight => ResizeResult {
            left_width: start_left_width,
            right_width: start_right_width,
            outer_gutter: clamp(
                start_outer_gutter.saturating_sub(delta),
                bounds.outer.0,
                bounds.outer.1,
            ),
        },
    }
}

pub(crate) fn start_resize_callback(
    set_is_resizing: WriteSignal<bool>,
    set_active_resize: WriteSignal<Option<ResizeTarget>>,
    set_start_x: WriteSignal<i32>,
    set_start_left_width: WriteSignal<i32>,
    set_start_right_width: WriteSignal<i32>,
    set_start_effective_left_width: WriteSignal<i32>,
    set_start_effective_right_width: WriteSignal<i32>,
    set_start_outer_gutter: WriteSignal<i32>,
    set_active_pointer: WriteSignal<Option<i32>>,
    current_left_width: Signal<i32>,
    current_right_width: Signal<i32>,
    current_effective_left_width: Signal<i32>,
    current_effective_right_width: Signal<i32>,
    current_outer_gutter: Signal<i32>,
    target: ResizeTarget,
) -> Callback<PointerEvent> {
    Callback::new(move |ev: PointerEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
        set_active_resize.set(Some(target));
        set_start_x.set(ev.client_x());
        set_start_left_width.set(current_left_width.get_untracked());
        set_start_right_width.set(current_right_width.get_untracked());
        set_start_effective_left_width.set(current_effective_left_width.get_untracked());
        set_start_effective_right_width.set(current_effective_right_width.get_untracked());
        set_start_outer_gutter.set(current_outer_gutter.get_untracked());
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
        if let Some(target) = state.active_resize.get_untracked() {
            let result = resized_values_for_target_with_constraints(
                target,
                state.start_left_width.get_untracked(),
                state.start_right_width.get_untracked(),
                state.start_effective_left_width.get_untracked(),
                state.start_effective_right_width.get_untracked(),
                state.start_outer_gutter.get_untracked(),
                delta,
                state.available_panel_width.get_untracked(),
                bounds,
            );
            outputs.set_left_width.set(result.left_width);
            outputs.set_right_width.set(result.right_width);
            outputs.set_outer_gutter.set(result.outer_gutter);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desktop_bounds() -> ResizeBounds {
        ResizeBounds {
            panel_width: (0, i32::MAX),
            outer: (0, 120),
        }
    }

    #[test]
    fn desktop_layout_resize_left_divider_collapses_sidebar_or_center() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_values_for_target(ResizeTarget::LeftDivider, 260, 320, 16, 40, 1000, bounds),
            ResizeResult {
                left_width: 300,
                right_width: 320,
                outer_gutter: 16,
            }
        );
        assert_eq!(
            resized_values_for_target(ResizeTarget::LeftDivider, 260, 320, 16, -400, 1000, bounds,),
            ResizeResult {
                left_width: 0,
                right_width: 320,
                outer_gutter: 16,
            }
        );
        assert_eq!(
            resized_values_for_target(ResizeTarget::LeftDivider, 260, 320, 16, 900, 1000, bounds),
            ResizeResult {
                left_width: 680,
                right_width: 320,
                outer_gutter: 16,
            }
        );
        assert_eq!(panel_center_width(680, 320, 1000), 0);
    }

    #[test]
    fn desktop_layout_resize_right_divider_collapses_chat_or_center() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_values_for_target(ResizeTarget::RightDivider, 260, 320, 16, -60, 1000, bounds),
            ResizeResult {
                left_width: 260,
                right_width: 380,
                outer_gutter: 16,
            }
        );
        assert_eq!(
            resized_values_for_target(ResizeTarget::RightDivider, 260, 320, 16, 500, 1000, bounds),
            ResizeResult {
                left_width: 260,
                right_width: 0,
                outer_gutter: 16,
            }
        );
        assert_eq!(
            resized_values_for_target(ResizeTarget::RightDivider, 260, 320, 16, -900, 1000, bounds,),
            ResizeResult {
                left_width: 260,
                right_width: 740,
                outer_gutter: 16,
            }
        );
        assert_eq!(panel_center_width(260, 740, 1000), 0);
    }

    #[test]
    fn desktop_layout_resize_preserves_existing_opposite_panel_width() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_values_for_target(ResizeTarget::LeftDivider, 250, 350, 16, 125, 1000, bounds),
            ResizeResult {
                left_width: 375,
                right_width: 350,
                outer_gutter: 16,
            }
        );
        assert_eq!(
            resized_values_for_target(ResizeTarget::RightDivider, 250, 350, 16, 125, 1000, bounds),
            ResizeResult {
                left_width: 250,
                right_width: 225,
                outer_gutter: 16,
            }
        );
    }

    #[test]
    fn desktop_layout_resize_hidden_regions_only_affect_constraints() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_values_for_target_with_constraints(
                ResizeTarget::LeftDivider,
                260,
                320,
                260,
                0,
                16,
                900,
                1000,
                bounds,
            ),
            ResizeResult {
                left_width: 1000,
                right_width: 320,
                outer_gutter: 16,
            }
        );
        assert_eq!(
            resized_values_for_target_with_constraints(
                ResizeTarget::RightDivider,
                260,
                320,
                0,
                320,
                16,
                -900,
                1000,
                bounds,
            ),
            ResizeResult {
                left_width: 260,
                right_width: 1000,
                outer_gutter: 16,
            }
        );
    }

    #[test]
    fn desktop_layout_resize_outer_gutter_uses_side_direction_and_clamps() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_values_for_target(ResizeTarget::OuterLeft, 250, 350, 48, 30, 1000, bounds),
            ResizeResult {
                left_width: 250,
                right_width: 350,
                outer_gutter: 78,
            }
        );
        assert_eq!(
            resized_values_for_target(ResizeTarget::OuterLeft, 250, 350, 48, -80, 1000, bounds),
            ResizeResult {
                left_width: 250,
                right_width: 350,
                outer_gutter: 0,
            }
        );
        assert_eq!(
            resized_values_for_target(ResizeTarget::OuterRight, 250, 350, 48, -30, 1000, bounds),
            ResizeResult {
                left_width: 250,
                right_width: 350,
                outer_gutter: 78,
            }
        );
        assert_eq!(
            resized_values_for_target(ResizeTarget::OuterRight, 250, 350, 48, 80, 1000, bounds),
            ResizeResult {
                left_width: 250,
                right_width: 350,
                outer_gutter: 0,
            }
        );
    }

    #[test]
    fn desktop_layout_resize_extreme_inputs_do_not_overflow() {
        let bounds = desktop_bounds();

        assert_eq!(
            resized_values_for_target(
                ResizeTarget::LeftDivider,
                i32::MAX,
                0,
                16,
                i32::MAX,
                1000,
                bounds,
            ),
            ResizeResult {
                left_width: 1000,
                right_width: 0,
                outer_gutter: 16,
            }
        );
        assert_eq!(
            resized_values_for_target(
                ResizeTarget::RightDivider,
                0,
                i32::MAX,
                16,
                i32::MIN,
                1000,
                bounds,
            ),
            ResizeResult {
                left_width: 0,
                right_width: 1000,
                outer_gutter: 16,
            }
        );
        assert_eq!(
            resized_values_for_target(
                ResizeTarget::OuterLeft,
                250,
                350,
                i32::MAX,
                i32::MAX,
                1000,
                bounds,
            )
            .outer_gutter,
            120
        );
        assert_eq!(
            resized_values_for_target(
                ResizeTarget::OuterRight,
                250,
                350,
                i32::MAX,
                i32::MIN,
                1000,
                bounds,
            )
            .outer_gutter,
            120
        );
    }
}
