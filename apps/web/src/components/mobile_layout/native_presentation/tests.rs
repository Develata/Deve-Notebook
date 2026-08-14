//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!

use super::*;

#[test]
fn mobile_drawer_edge_swipe_native_document_marker_fails_closed_and_web_defaults() {
    assert_eq!(initial_presentation_fallback(true), None);
    assert_eq!(
        initial_presentation_fallback(false),
        Some(SystemGestureInsets::web_default())
    );
}

#[test]
fn mobile_drawer_edge_swipe_presentation_order_rejects_stale_epochs() {
    let current = PresentationOrder {
        generation: 3,
        epoch: 8,
    };
    assert!(
        PresentationOrder {
            generation: 3,
            epoch: 9,
        } > current
    );
    assert!(
        PresentationOrder {
            generation: 2,
            epoch: 99,
        } < current
    );
}

#[test]
fn mobile_toolbar_keyboard_native_ime_inset_normalizes_physical_pixels() {
    let presentation = normalize_native_ime_presentation(4, true, 929.0, 2400.0, 2.75)
        .expect("valid current-generation IME geometry");
    assert_eq!(presentation.usable_offset(), 338);
    assert_eq!(presentation.generation(), 4);
}

#[test]
fn mobile_toolbar_keyboard_one_pixel_overlay_geometry_fails_closed() {
    let presentation = normalize_native_ime_presentation(4, true, 1.0, 2400.0, 2.75)
        .expect("one-pixel overlay remains a valid but unusable observation");
    assert_eq!(presentation.usable_offset(), 0);
    assert_eq!(
        normalize_native_ime_presentation(4, false, 929.0, 2400.0, 2.75)
            .expect("hidden IME geometry")
            .usable_offset(),
        0
    );
    assert!(normalize_native_ime_presentation(4, true, 2401.0, 2400.0, 2.75).is_none());
}

#[test]
fn mobile_safe_area_normalizes_native_physical_insets_and_rejects_invalid() {
    let safe_area = normalize_native_safe_area(94.0, 68.0, 2400.0, 2.75)
        .expect("valid Android system-bar safe area");
    assert_eq!(safe_area.top_css_px, 35);
    assert_eq!(safe_area.bottom_css_px, 25);
    assert!(normalize_native_safe_area(-1.0, 68.0, 2400.0, 2.75).is_none());
    assert!(normalize_native_safe_area(1200.0, 1201.0, 2400.0, 2.75).is_none());
}

#[test]
fn mobile_safe_area_css_uses_native_aware_shared_variables() {
    let css = include_str!("../../../../style/tailwind.css");
    assert!(css.contains("--deve-safe-area-top"));
    assert!(css.contains("--deve-safe-area-bottom"));
    assert!(css.contains("--deve-native-safe-area-top"));
    assert!(css.contains("--deve-native-safe-area-bottom"));

    let top_surfaces = [
        ("mobile header", include_str!("../header.rs")),
        ("chat sheet", include_str!("../chat_sheet.rs")),
        ("outline button", include_str!("../outline_button.rs")),
        ("drawer header helper", include_str!("../drawers/mod.rs")),
        ("chat header", include_str!("../../chat/header.rs")),
        (
            "search sheet",
            include_str!("../../search_box/ui_sheet/style.rs"),
        ),
        ("settings sheet", include_str!("../../settings.rs")),
        (
            "command palette",
            include_str!("../../command_palette/ui.rs"),
        ),
    ];
    for (surface, source) in top_surfaces {
        assert!(
            source.contains("--deve-safe-area-top"),
            "{surface} must consume the canonical top safe area"
        );
    }

    let bottom_surfaces = [
        ("mobile footer", include_str!("../footer.rs")),
        ("mobile toolbar", include_str!("../toolbar.rs")),
        ("chat sheet", include_str!("../chat_sheet.rs")),
        ("surface switcher", include_str!("../surface_switcher.rs")),
        ("left drawer body", include_str!("../drawers/left.rs")),
        ("right drawer body", include_str!("../drawers/right.rs")),
        ("chat input", include_str!("../../chat/input_area.rs")),
        (
            "repo removal dialog",
            include_str!("../../sidebar/repo_switcher/removal_dialog.rs"),
        ),
        ("settings sheet", include_str!("../../settings.rs")),
        (
            "command palette",
            include_str!("../../command_palette/ui.rs"),
        ),
    ];
    for (surface, source) in bottom_surfaces {
        assert!(
            source.contains("--deve-safe-area-bottom"),
            "{surface} must consume the canonical bottom safe area"
        );
    }

    let left_header = include_str!("../drawers/left/header.rs");
    let right_drawer = include_str!("../drawers/right.rs");
    assert!(left_header.contains("drawer_header_style()"));
    assert!(right_drawer.contains("drawer_header_style()"));
}
