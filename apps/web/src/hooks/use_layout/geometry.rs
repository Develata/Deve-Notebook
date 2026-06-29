//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Pure desktop layout width calculations.

use super::DESKTOP_DIVIDER_WIDTH;
use super::storage::clamp;

pub(crate) fn available_panel_width_for_viewport(
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

pub(crate) fn panel_center_width(left_width: i32, right_width: i32, available_width: i32) -> i32 {
    (available_width - left_width - right_width).max(0)
}

pub(crate) fn clamped_side_widths_for_visibility(
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

pub(crate) fn effective_region_width(width: i32, visible: bool) -> i32 {
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

pub(crate) fn initial_side_widths(
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
        available_panel_width_for_viewport, clamped_side_widths_for_visibility,
        effective_center_width, initial_side_widths, panel_center_width, visible_divider_count,
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

        let (default_left, default_right) = initial_side_widths(250, None, 350, 1168);
        assert_eq!(default_left, 250);
        assert_eq!(default_right, 350);
    }

    #[test]
    fn desktop_layout_resize_initial_widths_tolerate_extreme_persisted_values() {
        let (left, right) = initial_side_widths(i32::MAX, Some(0), i32::MAX, 1000);
        assert_eq!((left, right), (500, 500));

        let (left, right) = initial_side_widths(i32::MAX, Some(i32::MAX), i32::MAX, 1000);
        assert!((left as i64 + right as i64) <= 1000);
    }
}
