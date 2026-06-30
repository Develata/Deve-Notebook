// apps/web/src/hooks/use_layout.rs
//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Layout Hook for the desktop three-region splitter.

use self::geometry::{
    available_panel_width_for_viewport, clamped_side_widths_for_visibility, effective_region_width,
    initial_side_widths, panel_center_width,
};
use self::resize::{
    ResizeBounds, ResizeOutputSignals, ResizeStateSignals, ResizeTarget, do_resize_callback,
    start_resize_callback, stop_resize_callback,
};
use self::storage::{clamp, read_width, write_width};
use leptos::prelude::*;
use web_sys::PointerEvent;

mod geometry;
mod resize;
mod storage;

pub(crate) const DESKTOP_DIVIDER_WIDTH: i32 = 16;

const DEFAULT_DESKTOP_VIEWPORT_WIDTH: i32 = 1200;
const DEFAULT_LEFT_WIDTH: i32 = 250;
const DEFAULT_RIGHT_WIDTH: i32 = 350;
const DEFAULT_OUTER_GUTTER: i32 = 16;
const OUTER_GUTTER_MIN: i32 = 0;
const OUTER_GUTTER_MAX: i32 = 120;

#[derive(Clone, Copy)]
pub struct LayoutHookReturn {
    pub left_width: Signal<i32>,
    pub center_width: Signal<i32>,
    pub right_width: Signal<i32>,
    pub outer_gutter: Signal<i32>,
    pub start_left_divider_resize: Callback<PointerEvent>,
    pub start_right_divider_resize: Callback<PointerEvent>,
    pub start_outer_left_resize: Callback<PointerEvent>,
    pub start_outer_right_resize: Callback<PointerEvent>,
    pub stop_resize: Callback<()>,
    pub do_resize: Callback<PointerEvent>,
    pub is_resizing: ReadSignal<bool>,
}

pub fn use_layout(
    sidebar_visible: ReadSignal<bool>,
    chat_visible: ReadSignal<bool>,
) -> LayoutHookReturn {
    let (viewport_width, set_viewport_width) = signal(current_viewport_width());
    window_event_listener(leptos::ev::resize, move |_| {
        set_viewport_width.set(current_viewport_width());
    });

    let sidebar_initial = read_width("ui_sidebar_width").unwrap_or(DEFAULT_LEFT_WIDTH);
    let center_initial = read_width("ui_center_panel_width");
    let right_initial = read_width("ui_right_panel_width").unwrap_or(DEFAULT_RIGHT_WIDTH);
    let outer_initial = read_width("ui_outer_gutter").unwrap_or(DEFAULT_OUTER_GUTTER);
    let (outer_gutter, set_outer_gutter) =
        signal(clamp(outer_initial, OUTER_GUTTER_MIN, OUTER_GUTTER_MAX));
    let canonical_panel_width = Signal::derive(move || {
        available_panel_width_for_viewport(viewport_width.get(), outer_gutter.get(), true, true)
    });
    let available_panel_width = Signal::derive(move || {
        available_panel_width_for_viewport(
            viewport_width.get(),
            outer_gutter.get(),
            sidebar_visible.get(),
            chat_visible.get(),
        )
    });
    let (initial_left, initial_right) = initial_side_widths(
        sidebar_initial,
        center_initial,
        right_initial,
        canonical_panel_width.get_untracked(),
    );

    let (left_width, set_left_width) = signal(initial_left);
    let (right_width, set_right_width) = signal(initial_right);
    let (is_resizing, set_is_resizing) = signal(false);
    let (active_resize, set_active_resize) = signal(None::<ResizeTarget>);
    let (start_x, set_start_x) = signal(0i32);
    let (start_left_width, set_start_left_width) = signal(0i32);
    let (start_right_width, set_start_right_width) = signal(0i32);
    let (start_effective_left_width, set_start_effective_left_width) = signal(0i32);
    let (start_effective_right_width, set_start_effective_right_width) = signal(0i32);
    let (start_outer_gutter, set_start_outer_gutter) = signal(0i32);
    let (active_pointer, set_active_pointer) = signal(None::<i32>);
    let persisted_center_width = Signal::derive(move || {
        panel_center_width(
            left_width.get(),
            right_width.get(),
            canonical_panel_width.get(),
        )
    });
    let effective_left_width =
        Signal::derive(move || effective_region_width(left_width.get(), sidebar_visible.get()));
    let effective_right_width =
        Signal::derive(move || effective_region_width(right_width.get(), chat_visible.get()));
    let center_width = Signal::derive(move || {
        panel_center_width(
            effective_left_width.get(),
            effective_right_width.get(),
            available_panel_width.get(),
        )
    });

    Effect::new(move |_| {
        let available = available_panel_width.get();
        let left = left_width.get();
        let right = right_width.get();
        let (clamped_left, clamped_right) = clamped_side_widths_for_visibility(
            left,
            right,
            available,
            sidebar_visible.get(),
            chat_visible.get(),
        );

        if clamped_left != left {
            set_left_width.set(clamped_left);
        }
        if clamped_right != right {
            set_right_width.set(clamped_right);
        }
    });

    Effect::new(move |_| {
        write_width("ui_sidebar_width", left_width.get());
    });

    Effect::new(move |_| {
        write_width("ui_right_panel_width", right_width.get());
    });

    Effect::new(move |_| {
        write_width("ui_center_panel_width", persisted_center_width.get());
    });

    Effect::new(move |_| {
        write_width("ui_outer_gutter", outer_gutter.get());
    });

    let start_left_divider_resize = start_resize_callback(
        set_is_resizing,
        set_active_resize,
        set_start_x,
        set_start_left_width,
        set_start_right_width,
        set_start_effective_left_width,
        set_start_effective_right_width,
        set_start_outer_gutter,
        set_active_pointer,
        left_width.into(),
        right_width.into(),
        effective_left_width,
        effective_right_width,
        outer_gutter.into(),
        ResizeTarget::LeftDivider,
    );
    let start_right_divider_resize = start_resize_callback(
        set_is_resizing,
        set_active_resize,
        set_start_x,
        set_start_left_width,
        set_start_right_width,
        set_start_effective_left_width,
        set_start_effective_right_width,
        set_start_outer_gutter,
        set_active_pointer,
        left_width.into(),
        right_width.into(),
        effective_left_width,
        effective_right_width,
        outer_gutter.into(),
        ResizeTarget::RightDivider,
    );
    let start_outer_left_resize = start_resize_callback(
        set_is_resizing,
        set_active_resize,
        set_start_x,
        set_start_left_width,
        set_start_right_width,
        set_start_effective_left_width,
        set_start_effective_right_width,
        set_start_outer_gutter,
        set_active_pointer,
        left_width.into(),
        right_width.into(),
        effective_left_width,
        effective_right_width,
        outer_gutter.into(),
        ResizeTarget::OuterLeft,
    );
    let start_outer_right_resize = start_resize_callback(
        set_is_resizing,
        set_active_resize,
        set_start_x,
        set_start_left_width,
        set_start_right_width,
        set_start_effective_left_width,
        set_start_effective_right_width,
        set_start_outer_gutter,
        set_active_pointer,
        left_width.into(),
        right_width.into(),
        effective_left_width,
        effective_right_width,
        outer_gutter.into(),
        ResizeTarget::OuterRight,
    );
    let stop_resize = stop_resize_callback(set_is_resizing, set_active_resize, set_active_pointer);
    let do_resize = do_resize_callback(
        ResizeStateSignals {
            is_resizing,
            active_pointer,
            start_x,
            start_left_width,
            start_right_width,
            start_effective_left_width,
            start_effective_right_width,
            start_outer_gutter,
            active_resize,
            available_panel_width,
        },
        ResizeOutputSignals {
            set_left_width,
            set_right_width,
            set_outer_gutter,
        },
        ResizeBounds {
            panel_width: (0, i32::MAX),
            outer: (OUTER_GUTTER_MIN, OUTER_GUTTER_MAX),
        },
    );

    LayoutHookReturn {
        left_width: left_width.into(),
        center_width,
        right_width: right_width.into(),
        outer_gutter: outer_gutter.into(),
        start_left_divider_resize,
        start_right_divider_resize,
        start_outer_left_resize,
        start_outer_right_resize,
        stop_resize,
        do_resize,
        is_resizing,
    }
}

fn current_viewport_width() -> i32 {
    web_sys::window()
        .and_then(|w| w.inner_width().ok())
        .and_then(|v| v.as_f64())
        .map(|w| w.round() as i32)
        .unwrap_or(DEFAULT_DESKTOP_VIEWPORT_WIDTH)
}
