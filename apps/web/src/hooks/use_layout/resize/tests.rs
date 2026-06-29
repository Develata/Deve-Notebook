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
