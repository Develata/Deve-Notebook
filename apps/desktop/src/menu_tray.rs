//! plan_ref:
//!   - 17_tech_stack#native-packaging-dependency-gate
//!   - 11_ui_design/02_desktop#desktop-packaging-scaffold
//!
//! Tauri menu/tray binding for the desktop native-packaging feature.
//!
//! This module only binds shell UI intents to Tauri menu/tray builders. It does
//! not start a Tauri runtime, spawn a backend process, or write business state.

use crate::{
    DESKTOP_MENU_APP_ID, DESKTOP_MENU_HELP_ID, DESKTOP_MENU_WINDOW_ID, DESKTOP_TAURI_PRODUCT_NAME,
    DESKTOP_TRAY_ID, DesktopMenuAction, DesktopTrayAction,
};
use tauri::menu::{Menu, MenuBuilder, SubmenuBuilder};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{Manager, Runtime};

pub const DESKTOP_MENU_ROOT_ID: &str = "deve-menu-root";
pub const DESKTOP_MENU_APP_SHOW_MAIN_WINDOW_ID: &str = "deve.menu.app.show_main_window";
pub const DESKTOP_MENU_APP_OPEN_COMMAND_PALETTE_ID: &str = "deve.menu.app.open_command_palette";
pub const DESKTOP_MENU_APP_OPEN_SETTINGS_ID: &str = "deve.menu.app.open_settings";
pub const DESKTOP_MENU_APP_QUIT_REQUESTED_ID: &str = "deve.menu.app.quit_requested";
pub const DESKTOP_MENU_WINDOW_SHOW_MAIN_WINDOW_ID: &str = "deve.menu.window.show_main_window";
pub const DESKTOP_MENU_HELP_OPEN_COMMAND_PALETTE_ID: &str = "deve.menu.help.open_command_palette";

pub const DESKTOP_TRAY_MENU_ID: &str = "deve-tray-menu";
pub const DESKTOP_TRAY_SHOW_MAIN_WINDOW_ID: &str = "deve.tray.show_main_window";
pub const DESKTOP_TRAY_TOGGLE_WINDOW_VISIBILITY_ID: &str = "deve.tray.toggle_window_visibility";
pub const DESKTOP_TRAY_QUIT_REQUESTED_ID: &str = "deve.tray.quit_requested";

pub fn build_desktop_menu<R, M>(manager: &M) -> tauri::Result<Menu<R>>
where
    R: Runtime,
    M: Manager<R>,
{
    let app_menu = SubmenuBuilder::with_id(manager, DESKTOP_MENU_APP_ID, "Deve")
        .text(DESKTOP_MENU_APP_SHOW_MAIN_WINDOW_ID, "Show Main Window")
        .text(DESKTOP_MENU_APP_OPEN_COMMAND_PALETTE_ID, "Command Palette")
        .text(DESKTOP_MENU_APP_OPEN_SETTINGS_ID, "Settings")
        .separator()
        .text(DESKTOP_MENU_APP_QUIT_REQUESTED_ID, "Quit")
        .build()?;
    let window_menu = SubmenuBuilder::with_id(manager, DESKTOP_MENU_WINDOW_ID, "Window")
        .text(DESKTOP_MENU_WINDOW_SHOW_MAIN_WINDOW_ID, "Show Main Window")
        .build()?;
    let help_menu = SubmenuBuilder::with_id(manager, DESKTOP_MENU_HELP_ID, "Help")
        .text(
            DESKTOP_MENU_HELP_OPEN_COMMAND_PALETTE_ID,
            "Open Command Palette",
        )
        .build()?;

    MenuBuilder::with_id(manager, DESKTOP_MENU_ROOT_ID)
        .items(&[&app_menu, &window_menu, &help_menu])
        .build()
}

pub fn build_desktop_tray_menu<R, M>(manager: &M) -> tauri::Result<Menu<R>>
where
    R: Runtime,
    M: Manager<R>,
{
    MenuBuilder::with_id(manager, DESKTOP_TRAY_MENU_ID)
        .text(DESKTOP_TRAY_SHOW_MAIN_WINDOW_ID, "Show Main Window")
        .text(
            DESKTOP_TRAY_TOGGLE_WINDOW_VISIBILITY_ID,
            "Toggle Window Visibility",
        )
        .separator()
        .text(DESKTOP_TRAY_QUIT_REQUESTED_ID, "Quit")
        .build()
}

pub fn build_desktop_tray_icon<R, M>(manager: &M, tray_menu: &Menu<R>) -> tauri::Result<TrayIcon<R>>
where
    R: Runtime,
    M: Manager<R>,
{
    TrayIconBuilder::with_id(DESKTOP_TRAY_ID)
        .menu(tray_menu)
        .tooltip(DESKTOP_TAURI_PRODUCT_NAME)
        .show_menu_on_left_click(true)
        .build(manager)
}

pub fn resolve_desktop_menu_action_id(id: &str) -> Option<DesktopMenuAction> {
    match id {
        DESKTOP_MENU_APP_SHOW_MAIN_WINDOW_ID | DESKTOP_MENU_WINDOW_SHOW_MAIN_WINDOW_ID => {
            Some(DesktopMenuAction::ShowMainWindow)
        }
        DESKTOP_MENU_APP_OPEN_COMMAND_PALETTE_ID | DESKTOP_MENU_HELP_OPEN_COMMAND_PALETTE_ID => {
            Some(DesktopMenuAction::OpenCommandPalette)
        }
        DESKTOP_MENU_APP_OPEN_SETTINGS_ID => Some(DesktopMenuAction::OpenSettings),
        DESKTOP_MENU_APP_QUIT_REQUESTED_ID => Some(DesktopMenuAction::QuitRequested),
        _ => None,
    }
}

pub fn resolve_desktop_tray_action_id(id: &str) -> Option<DesktopTrayAction> {
    match id {
        DESKTOP_TRAY_SHOW_MAIN_WINDOW_ID => Some(DesktopTrayAction::ShowMainWindow),
        DESKTOP_TRAY_TOGGLE_WINDOW_VISIBILITY_ID => Some(DesktopTrayAction::ToggleWindowVisibility),
        DESKTOP_TRAY_QUIT_REQUESTED_ID => Some(DesktopTrayAction::QuitRequested),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_menu_action_ids_resolve_to_ui_intents() {
        assert_eq!(
            resolve_desktop_menu_action_id(DESKTOP_MENU_APP_SHOW_MAIN_WINDOW_ID),
            Some(DesktopMenuAction::ShowMainWindow)
        );
        assert_eq!(
            resolve_desktop_menu_action_id(DESKTOP_MENU_WINDOW_SHOW_MAIN_WINDOW_ID),
            Some(DesktopMenuAction::ShowMainWindow)
        );
        assert_eq!(
            resolve_desktop_menu_action_id(DESKTOP_MENU_APP_OPEN_COMMAND_PALETTE_ID),
            Some(DesktopMenuAction::OpenCommandPalette)
        );
        assert_eq!(
            resolve_desktop_menu_action_id(DESKTOP_MENU_APP_OPEN_SETTINGS_ID),
            Some(DesktopMenuAction::OpenSettings)
        );
        assert_eq!(
            resolve_desktop_menu_action_id(DESKTOP_MENU_APP_QUIT_REQUESTED_ID),
            Some(DesktopMenuAction::QuitRequested)
        );
        assert_eq!(resolve_desktop_menu_action_id("deve.menu.unknown"), None);
    }

    #[test]
    fn desktop_tray_action_ids_resolve_to_ui_intents() {
        assert_eq!(
            resolve_desktop_tray_action_id(DESKTOP_TRAY_SHOW_MAIN_WINDOW_ID),
            Some(DesktopTrayAction::ShowMainWindow)
        );
        assert_eq!(
            resolve_desktop_tray_action_id(DESKTOP_TRAY_TOGGLE_WINDOW_VISIBILITY_ID),
            Some(DesktopTrayAction::ToggleWindowVisibility)
        );
        assert_eq!(
            resolve_desktop_tray_action_id(DESKTOP_TRAY_QUIT_REQUESTED_ID),
            Some(DesktopTrayAction::QuitRequested)
        );
        assert_eq!(resolve_desktop_tray_action_id("deve.tray.unknown"), None);
    }
}
