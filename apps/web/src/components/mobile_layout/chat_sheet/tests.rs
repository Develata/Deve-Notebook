//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 16_ai_agent#native-ai-chat-runtime

use super::{
    mobile_chat_after_close, mobile_chat_after_open, mobile_chat_page_mode,
    mobile_chat_runtime_conflict_should_close, mobile_chat_sheet_class, mobile_chat_sheet_style,
    should_show_mobile_chat_sheet,
};

#[test]
fn expanded_chat_stays_visible_when_keyboard_is_open() {
    assert!(should_show_mobile_chat_sheet(
        true, false, false, false, true, true
    ));
    let style = mobile_chat_sheet_style(true, 280);
    assert!(style.contains("padding-top: var(--deve-safe-area-top);"));
    assert!(style.contains("bottom: 280px;"));
}

#[test]
fn mobile_chat_keyboard_sheet_stays_above_keyboard() {
    assert!(should_show_mobile_chat_sheet(
        true, false, false, false, true, true
    ));
    assert!(mobile_chat_sheet_style(true, 320).contains("bottom: 320px;"));
}

#[test]
fn collapsed_chip_hides_when_keyboard_is_open() {
    assert!(!should_show_mobile_chat_sheet(
        true, false, false, false, false, true
    ));
}

#[test]
fn drawer_and_diff_still_hide_mobile_chat() {
    assert!(!should_show_mobile_chat_sheet(
        true, true, false, false, true, false
    ));
    assert!(!should_show_mobile_chat_sheet(
        true, false, true, false, true, false
    ));
}

#[test]
fn mobile_diff_hides_chat_chip_and_expanded_chat() {
    assert!(!should_show_mobile_chat_sheet(
        true, false, true, false, false, false
    ));
    assert!(!should_show_mobile_chat_sheet(
        true, false, true, false, true, false
    ));
}

#[test]
fn mobile_surface_switcher_hides_chat_sheet() {
    assert!(!should_show_mobile_chat_sheet(
        true, false, false, true, false, false
    ));
    assert!(!should_show_mobile_chat_sheet(
        true, false, false, true, true, false
    ));
}

#[test]
fn collapsed_chip_uses_footer_offset_when_keyboard_is_closed() {
    assert!(should_show_mobile_chat_sheet(
        true, false, false, false, false, false
    ));
    assert_eq!(
        mobile_chat_sheet_style(false, 0),
        "bottom: calc(58px + var(--deve-safe-area-bottom));"
    );
}

#[test]
fn mobile_chat_page_expands_to_fullscreen() {
    assert_eq!(mobile_chat_page_mode(true), "fullscreen");
    assert!(mobile_chat_sheet_class(true).contains("fixed inset-0"));
    assert!(mobile_chat_sheet_class(true).contains("z-[var(--z-overlay)]"));
    assert!(mobile_chat_sheet_style(true, 0).contains("--deve-safe-area-top"));
}

#[test]
fn mobile_chat_page_close_returns_to_editor_surface() {
    assert_eq!(mobile_chat_page_mode(false), "chip");
    assert!(mobile_chat_sheet_class(false).contains("z-[var(--z-floating)]"));
    assert!(mobile_chat_after_open());
    assert!(!mobile_chat_after_close());
}

#[test]
fn mobile_chat_runtime_conflicts_close_expanded_page() {
    assert!(mobile_chat_runtime_conflict_should_close(
        false, false, false, false, true
    ));
    assert!(mobile_chat_runtime_conflict_should_close(
        true, true, false, false, true
    ));
    assert!(mobile_chat_runtime_conflict_should_close(
        true, false, true, false, true
    ));
    assert!(mobile_chat_runtime_conflict_should_close(
        true, false, false, true, true
    ));
    assert!(!mobile_chat_runtime_conflict_should_close(
        true, false, false, false, true
    ));
    assert!(!mobile_chat_runtime_conflict_should_close(
        true, true, false, false, false
    ));
}
