use super::{
    mobile_accessory_toolbar_visible, mobile_bottom_bar_visible,
    should_clear_mobile_source_control_local_notice,
};
use crate::components::activity_bar::SidebarView;
use crate::components::mobile_layout::surface_switcher::mobile_surface_sheet_visible;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use deve_core::protocol::ServerErrorCode;

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

#[test]
fn mobile_menu_open_clears_stale_source_control_local_command_notice() {
    let server_notice = SourceControlNotice {
        code: ServerErrorCode::ScDocNotFound,
        detail: None,
    };

    for local_notice in [
        SourceControlNotice::git_status_cli_only(),
        SourceControlNotice::git_mirror_cli_only(),
        SourceControlNotice::git_export_cli_only(),
        SourceControlNotice::git_import_cli_only(),
        SourceControlNotice::git_push_cli_only(),
        SourceControlNotice::git_repair_cli_only(),
        SourceControlNotice::establish_branch_unavailable(),
    ] {
        assert!(should_clear_mobile_source_control_local_notice(
            SidebarView::SourceControl,
            Some(&local_notice),
        ));
        assert!(!should_clear_mobile_source_control_local_notice(
            SidebarView::Explorer,
            Some(&local_notice),
        ));
    }
    assert!(!should_clear_mobile_source_control_local_notice(
        SidebarView::SourceControl,
        Some(&server_notice),
    ));
    assert!(!should_clear_mobile_source_control_local_notice(
        SidebarView::SourceControl,
        None,
    ));
}
