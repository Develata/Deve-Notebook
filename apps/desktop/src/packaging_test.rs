//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate

use serde_json::Value;

use crate::{
    DESKTOP_MENU_APP_ID, DESKTOP_MENU_HELP_ID, DESKTOP_MENU_WINDOW_ID, DESKTOP_TAURI_CONFIG_PATH,
    DESKTOP_TAURI_IDENTIFIER, DESKTOP_TAURI_MAIN_WINDOW_LABEL, DESKTOP_TAURI_MAIN_WINDOW_TITLE,
    DESKTOP_TAURI_PRODUCT_NAME, DESKTOP_TRAY_ID, DesktopMenuAction, DesktopPackagingAuthority,
    DesktopPackagingCapability, DesktopTrayAction, desktop_packaging_scaffold,
};
use deve_core::native_adapter::CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

#[test]
fn desktop_packaging_dependency_spike_is_feature_gated() {
    let scaffold = desktop_packaging_scaffold();
    let gate = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

    assert_eq!(scaffold.dependency_batch.feature_gate, "native-packaging");
    assert_eq!(scaffold.dependency_batch.runtime_crate, "tauri");
    assert_eq!(scaffold.dependency_batch.build_crate, "tauri-build");
    assert_eq!(scaffold.dependency_batch.status, "dependency-spike-open");
    assert!(scaffold.dependency_feature_is_isolated());
    assert!(gate.is_desktop_dependency_spike_open());
    assert!(gate.desktop_tauri_dependencies_allowed);
    assert!(!gate.mobile_tauri_dependencies_allowed);
}

#[test]
fn desktop_packaging_acceptance_is_shell_only() {
    let scaffold = desktop_packaging_scaffold();

    assert_eq!(
        scaffold.acceptance.capabilities,
        [
            DesktopPackagingCapability::WindowShell,
            DesktopPackagingCapability::MenuBar,
            DesktopPackagingCapability::SystemTray,
            DesktopPackagingCapability::Installer,
            DesktopPackagingCapability::AutoUpdate,
        ]
    );
    assert_eq!(
        scaffold.acceptance.forbidden_authorities,
        [
            DesktopPackagingAuthority::Ledger,
            DesktopPackagingAuthority::Vault,
            DesktopPackagingAuthority::SourceControl,
            DesktopPackagingAuthority::SearchIndex,
            DesktopPackagingAuthority::GitMirror,
            DesktopPackagingAuthority::NoteGit,
        ]
    );
    assert!(scaffold.is_authority_free());
    assert!(scaffold.shell_acceptance_is_authority_free());
    assert!(scaffold.no_packaging_tests_remain_authoritative);
}

#[test]
fn desktop_tauri_manifest_declares_shell_metadata_only() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json"))
        .expect("desktop tauri config should be valid json");
    let scaffold = desktop_packaging_scaffold();
    let shell = scaffold.acceptance.shell;
    let window = &config["app"]["windows"][0];

    assert_eq!(shell.tauri_config_path, DESKTOP_TAURI_CONFIG_PATH);
    assert_eq!(config["productName"], DESKTOP_TAURI_PRODUCT_NAME);
    assert_eq!(config["identifier"], DESKTOP_TAURI_IDENTIFIER);
    assert_eq!(window["label"], DESKTOP_TAURI_MAIN_WINDOW_LABEL);
    assert_eq!(window["title"], DESKTOP_TAURI_MAIN_WINDOW_TITLE);
    assert_eq!(window["resizable"], true);
    assert_eq!(window["fullscreen"], false);
    assert_eq!(config["app"]["withGlobalTauri"], false);
    assert_eq!(config["bundle"]["active"], true);
    assert_eq!(config["bundle"]["createUpdaterArtifacts"], false);
    assert!(config["bundle"]["targets"].is_null());
    assert!(
        config["plugins"]
            .as_object()
            .is_some_and(|plugins| plugins.is_empty())
    );
}

#[test]
fn desktop_shell_acceptance_keeps_runtime_authority_closed() {
    let shell = desktop_packaging_scaffold().acceptance.shell;

    assert_eq!(shell.product_name, DESKTOP_TAURI_PRODUCT_NAME);
    assert_eq!(shell.identifier, DESKTOP_TAURI_IDENTIFIER);
    assert_eq!(shell.main_window_label, DESKTOP_TAURI_MAIN_WINDOW_LABEL);
    assert_eq!(shell.main_window_title, DESKTOP_TAURI_MAIN_WINDOW_TITLE);
    assert!(!shell.menu_bar_runtime_declared);
    assert!(!shell.system_tray_runtime_declared);
    assert!(shell.menu_and_tray_runtime_deferred);
    assert!(shell.installer_metadata_declared);
    assert!(!shell.auto_update_artifacts_enabled);
    assert!(shell.session_handoff_required_before_writable_ui);
    assert!(!shell.child_process_runtime_enabled);
    assert!(!shell.release_ready_claimed);
}

#[test]
fn desktop_menu_tray_surface_declares_ui_intents_only() {
    let surface = desktop_packaging_scaffold().acceptance.menu_tray;

    assert_eq!(surface.app_menu_id, DESKTOP_MENU_APP_ID);
    assert_eq!(surface.window_menu_id, DESKTOP_MENU_WINDOW_ID);
    assert_eq!(surface.help_menu_id, DESKTOP_MENU_HELP_ID);
    assert_eq!(surface.tray_id, DESKTOP_TRAY_ID);
    assert_eq!(
        surface.menu_actions,
        [
            DesktopMenuAction::ShowMainWindow,
            DesktopMenuAction::OpenCommandPalette,
            DesktopMenuAction::OpenSettings,
            DesktopMenuAction::QuitRequested,
        ]
    );
    assert_eq!(
        surface.tray_actions,
        [
            DesktopTrayAction::ShowMainWindow,
            DesktopTrayAction::ToggleWindowVisibility,
            DesktopTrayAction::QuitRequested,
        ]
    );
    assert!(surface.is_deferred_runtime_surface());
    assert!(!surface.menu_runtime_imported);
    assert!(!surface.tray_runtime_imported);
    assert!(surface.actions_are_ui_intents_only);
    assert!(!surface.opens_process_runtime);
    assert!(!surface.opens_authority_write_path);
}
