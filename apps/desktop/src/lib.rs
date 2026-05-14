//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!
//! Minimal desktop native shell skeleton.
//!
//! The default build intentionally avoids Tauri; optional native packaging
//! dependencies are isolated behind the `native-packaging` feature. This crate
//! models the shell's allowed responsibilities: bind to a controlled local
//! service, bind a short-lived session out of band, inject a Web bootstrap
//! object, and report service recovery state.

#[cfg(feature = "native-packaging")]
mod menu_tray;
#[cfg(feature = "native-packaging")]
mod packaging;
#[cfg(all(test, feature = "native-packaging"))]
mod packaging_test;
mod shell;
#[cfg(test)]
mod shell_recovery_test;
#[cfg(test)]
mod shell_test;
mod types;

#[cfg(feature = "native-packaging")]
pub use menu_tray::{
    DESKTOP_MENU_APP_OPEN_COMMAND_PALETTE_ID, DESKTOP_MENU_APP_OPEN_SETTINGS_ID,
    DESKTOP_MENU_APP_QUIT_REQUESTED_ID, DESKTOP_MENU_APP_SHOW_MAIN_WINDOW_ID,
    DESKTOP_MENU_HELP_OPEN_COMMAND_PALETTE_ID, DESKTOP_MENU_ROOT_ID,
    DESKTOP_MENU_WINDOW_SHOW_MAIN_WINDOW_ID, DESKTOP_TRAY_MENU_ID, DESKTOP_TRAY_QUIT_REQUESTED_ID,
    DESKTOP_TRAY_SHOW_MAIN_WINDOW_ID, DESKTOP_TRAY_TOGGLE_WINDOW_VISIBILITY_ID, build_desktop_menu,
    build_desktop_tray_icon, build_desktop_tray_menu, resolve_desktop_menu_action_id,
    resolve_desktop_tray_action_id,
};
#[cfg(feature = "native-packaging")]
pub use packaging::{
    DESKTOP_MENU_APP_ID, DESKTOP_MENU_HELP_ID, DESKTOP_MENU_WINDOW_ID, DESKTOP_TAURI_CONFIG_PATH,
    DESKTOP_TAURI_IDENTIFIER, DESKTOP_TAURI_MAIN_WINDOW_LABEL, DESKTOP_TAURI_MAIN_WINDOW_TITLE,
    DESKTOP_TAURI_PRODUCT_NAME, DESKTOP_TRAY_ID, DesktopMenuAction, DesktopMenuTraySurface,
    DesktopPackagingAcceptance, DesktopPackagingAuthority, DesktopPackagingCapability,
    DesktopPackagingDependencyBatch, DesktopPackagingScaffold, DesktopShellPackagingAcceptance,
    DesktopTrayAction, desktop_packaging_scaffold,
};
pub use shell::DesktopShell;
pub use types::{
    DesktopBootstrap, DesktopRecoveryBootstrap, DesktopServiceState, DesktopSessionMaterial,
    DesktopShellError, DesktopShellSnapshot,
};
