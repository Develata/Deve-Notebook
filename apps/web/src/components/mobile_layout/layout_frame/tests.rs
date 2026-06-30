use super::{mobile_accessory_toolbar_visible, mobile_bottom_bar_visible};
use crate::components::mobile_layout::surface_switcher::mobile_surface_sheet_visible;

#[test]
fn mobile_chat_keyboard_hides_bottom_bar() {
    assert!(!mobile_bottom_bar_visible(280, true, false));
    assert!(!mobile_bottom_bar_visible(280, false, false));
    assert!(!mobile_bottom_bar_visible(0, true, false));
    assert!(mobile_bottom_bar_visible(0, false, false));
    assert!(!mobile_bottom_bar_visible(0, false, true));
}

#[test]
fn mobile_surface_switcher_hides_bottom_bar() {
    assert!(!mobile_bottom_bar_visible(0, false, true));
    assert!(mobile_bottom_bar_visible(0, false, false));
}

#[test]
fn mobile_surface_shell_chrome_uses_sheet_visibility_not_raw_open_state() {
    let raw_open_but_drawer_hides_sheet = mobile_surface_sheet_visible(true, true, true);
    let raw_open_but_empty_tabs_hide_sheet = mobile_surface_sheet_visible(true, false, false);
    let visible_sheet = mobile_surface_sheet_visible(true, false, true);

    assert!(!raw_open_but_drawer_hides_sheet);
    assert!(!raw_open_but_empty_tabs_hide_sheet);
    assert!(visible_sheet);

    assert!(mobile_bottom_bar_visible(
        0,
        false,
        raw_open_but_drawer_hides_sheet
    ));
    assert!(mobile_bottom_bar_visible(
        0,
        false,
        raw_open_but_empty_tabs_hide_sheet
    ));
    assert!(!mobile_bottom_bar_visible(0, false, visible_sheet));

    assert!(mobile_accessory_toolbar_visible(
        true,
        false,
        false,
        280,
        false,
        raw_open_but_drawer_hides_sheet,
    ));
    assert!(mobile_accessory_toolbar_visible(
        true,
        false,
        false,
        280,
        false,
        raw_open_but_empty_tabs_hide_sheet,
    ));
    assert!(!mobile_accessory_toolbar_visible(
        true,
        false,
        false,
        280,
        false,
        visible_sheet,
    ));
}

#[test]
fn mobile_diff_hides_accessory_toolbar() {
    assert!(mobile_accessory_toolbar_visible(
        true, false, false, 280, false, false
    ));
    assert!(!mobile_accessory_toolbar_visible(
        true, true, false, 280, false, false
    ));
}

#[test]
fn mobile_diff_keeps_accessory_toolbar_gate_strict() {
    assert!(!mobile_accessory_toolbar_visible(
        false, false, false, 280, false, false
    ));
    assert!(!mobile_accessory_toolbar_visible(
        true, false, true, 280, false, false
    ));
    assert!(!mobile_accessory_toolbar_visible(
        true, false, false, 0, false, false
    ));
    assert!(!mobile_accessory_toolbar_visible(
        true, false, false, 280, true, false
    ));
    assert!(!mobile_accessory_toolbar_visible(
        true, false, false, 280, false, true
    ));
}
