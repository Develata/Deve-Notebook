// apps/web/src/hooks/use_layout.rs
//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Layout Hook for the desktop three-region splitter.

use self::resize::{
    ResizeBounds, ResizeOutputSignals, ResizeStateSignals, ResizeTarget, do_resize_callback,
    start_resize_callback, stop_resize_callback,
};
use self::storage::{clamp, read_width, write_width};
use leptos::prelude::*;
use web_sys::PointerEvent;

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

fn available_panel_width_for_viewport(
    viewport_width: i32,
    outer_gutter: i32,
    sidebar_visible: bool,
    chat_visible: bool,
) -> i32 {
    let divider_width =
        DESKTOP_DIVIDER_WIDTH * visible_divider_count(sidebar_visible, chat_visible);
    (viewport_width - (outer_gutter.max(0) * 2) - divider_width).max(0)
}

fn visible_divider_count(sidebar_visible: bool, chat_visible: bool) -> i32 {
    let sidebar_divider = if sidebar_visible { 1 } else { 0 };
    let chat_divider = if chat_visible { 1 } else { 0 };
    sidebar_divider + chat_divider
}

fn panel_center_width(left_width: i32, right_width: i32, available_width: i32) -> i32 {
    (available_width - left_width - right_width).max(0)
}

fn clamped_side_widths_for_visibility(
    left_width: i32,
    right_width: i32,
    available_width: i32,
    sidebar_visible: bool,
    chat_visible: bool,
) -> (i32, i32) {
    match (sidebar_visible, chat_visible) {
        (true, true) => {
            let clamped_left = clamp(left_width, 0, available_width);
            let clamped_right = clamp(right_width, 0, (available_width - clamped_left).max(0));
            (clamped_left, clamped_right)
        }
        (true, false) => (clamp(left_width, 0, available_width), right_width.max(0)),
        (false, true) => (left_width.max(0), clamp(right_width, 0, available_width)),
        (false, false) => (left_width.max(0), right_width.max(0)),
    }
}

fn effective_region_width(width: i32, visible: bool) -> i32 {
    if visible { width } else { 0 }
}

#[cfg(test)]
fn effective_center_width(
    left_width: i32,
    right_width: i32,
    available_width: i32,
    sidebar_visible: bool,
    chat_visible: bool,
) -> i32 {
    panel_center_width(
        effective_region_width(left_width, sidebar_visible),
        effective_region_width(right_width, chat_visible),
        available_width,
    )
}

fn initial_side_widths(
    left_width: i32,
    center_width: Option<i32>,
    right_width: i32,
    available_width: i32,
) -> (i32, i32) {
    let left = i64::from(left_width.max(0));
    let right = i64::from(right_width.max(0));
    let center = i64::from(
        center_width
            .unwrap_or_else(|| {
                panel_center_width(left_width.max(0), right_width.max(0), available_width)
            })
            .max(0),
    );
    let available = i64::from(available_width.max(0));
    let total = left + center + right;

    if total <= 0 || available <= 0 {
        return (0, 0);
    }

    let scaled_left = ((left * available) / total) as i32;
    let mut scaled_right = ((right * available) / total) as i32;
    if scaled_left.saturating_add(scaled_right) > available_width {
        scaled_right = available_width.saturating_sub(scaled_left).max(0);
    }

    (scaled_left, scaled_right)
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_LEFT_WIDTH, DEFAULT_RIGHT_WIDTH, available_panel_width_for_viewport,
        clamped_side_widths_for_visibility, effective_center_width, initial_side_widths,
        panel_center_width, visible_divider_count,
    };

    #[test]
    fn desktop_layout_resize_available_width_reserves_divider_tracks_and_outer_gutter() {
        assert_eq!(visible_divider_count(true, true), 2);
        assert_eq!(visible_divider_count(true, false), 1);
        assert_eq!(visible_divider_count(false, true), 1);
        assert_eq!(visible_divider_count(false, false), 0);
        assert_eq!(
            available_panel_width_for_viewport(1000, 16, true, true),
            936
        );
        assert_eq!(
            available_panel_width_for_viewport(1000, 16, true, false),
            952
        );
        assert_eq!(
            available_panel_width_for_viewport(1000, 16, false, false),
            968
        );
        assert_eq!(available_panel_width_for_viewport(20, 16, true, true), 0);
    }

    #[test]
    fn desktop_layout_resize_center_width_is_derived_from_side_widths() {
        assert_eq!(panel_center_width(250, 350, 1000), 400);
        assert_eq!(panel_center_width(700, 350, 1000), 0);
    }

    #[test]
    fn desktop_layout_resize_effective_center_ignores_hidden_regions() {
        assert_eq!(effective_center_width(650, 350, 1000, true, true), 0);
        assert_eq!(effective_center_width(650, 350, 1000, true, false), 350);
        assert_eq!(effective_center_width(650, 350, 1000, false, true), 650);
        assert_eq!(effective_center_width(650, 350, 1000, false, false), 1000);
    }

    #[test]
    fn desktop_layout_resize_clamp_preserves_hidden_region_widths() {
        assert_eq!(
            clamped_side_widths_for_visibility(952, 350, 952, true, false),
            (952, 350)
        );
        assert_eq!(
            clamped_side_widths_for_visibility(250, 952, 952, false, true),
            (250, 952)
        );
        assert_eq!(
            clamped_side_widths_for_visibility(700, 500, 936, true, true),
            (700, 236)
        );
    }

    #[test]
    fn desktop_layout_resize_initial_widths_use_persisted_center_ratio() {
        let (left, right) = initial_side_widths(250, Some(500), 250, 1000);
        assert_eq!(left, 250);
        assert_eq!(right, 250);

        let (default_left, default_right) =
            initial_side_widths(DEFAULT_LEFT_WIDTH, None, DEFAULT_RIGHT_WIDTH, 1168);
        assert_eq!(default_left, DEFAULT_LEFT_WIDTH);
        assert_eq!(default_right, DEFAULT_RIGHT_WIDTH);
    }

    #[test]
    fn desktop_layout_resize_initial_widths_tolerate_extreme_persisted_values() {
        let (left, right) = initial_side_widths(i32::MAX, Some(0), i32::MAX, 1000);
        assert_eq!((left, right), (500, 500));

        let (left, right) = initial_side_widths(i32::MAX, Some(i32::MAX), i32::MAX, 1000);
        assert!((left as i64 + right as i64) <= 1000);
    }
}
