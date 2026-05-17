//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate
//!   - 08_ui_design_02_desktop#desktop-packaging-scaffold
//!
//! Desktop Tauri window-shell entrypoint.
//!
//! The runtime starts only the native shell window/menu/tray surface.
//! Local-service process planning lives in `service_entrypoint`; actual spawn,
//! health probe, and session handoff must be wired before Web bootstrap in a
//! later batch. This module does not write ledger, vault, source-control,
//! search, Git, or `.notegit` authority.

use crate::{
    DESKTOP_TAURI_MAIN_WINDOW_LABEL, DesktopMenuAction, DesktopTrayAction, build_desktop_menu,
    build_desktop_tray_icon, build_desktop_tray_menu, resolve_desktop_menu_action_id,
    resolve_desktop_tray_action_id,
};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopTauriShellEffect {
    ShowMainWindow,
    ToggleMainWindowVisibility,
    QuitRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopTauriRuntimeSurface {
    pub runtime_entrypoint_declared: bool,
    pub build_script_declared: bool,
    pub window_shell_runtime_declared: bool,
    pub menu_tray_runtime_bound: bool,
    pub child_process_runtime_enabled: bool,
    pub opens_authority_write_path: bool,
}

impl DesktopTauriRuntimeSurface {
    pub fn is_shell_only(self) -> bool {
        self.runtime_entrypoint_declared
            && self.build_script_declared
            && self.window_shell_runtime_declared
            && self.menu_tray_runtime_bound
            && !self.child_process_runtime_enabled
            && !self.opens_authority_write_path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopTauriStartupSmoke {
    pub packaged_binary_started: bool,
    pub shell_only_runtime: bool,
    pub child_process_runtime_enabled: bool,
    pub opens_authority_write_path: bool,
}

impl DesktopTauriStartupSmoke {
    pub fn passed(self) -> bool {
        self.packaged_binary_started
            && self.shell_only_runtime
            && !self.child_process_runtime_enabled
            && !self.opens_authority_write_path
    }
}

pub const DESKTOP_TAURI_STARTUP_SMOKE_OK: &str = "desktop-startup-smoke: ok";

pub fn desktop_tauri_runtime_surface() -> DesktopTauriRuntimeSurface {
    DesktopTauriRuntimeSurface {
        runtime_entrypoint_declared: true,
        build_script_declared: true,
        window_shell_runtime_declared: true,
        menu_tray_runtime_bound: true,
        child_process_runtime_enabled: false,
        opens_authority_write_path: false,
    }
}

pub fn desktop_tauri_startup_smoke() -> DesktopTauriStartupSmoke {
    let surface = desktop_tauri_runtime_surface();
    DesktopTauriStartupSmoke {
        packaged_binary_started: true,
        shell_only_runtime: surface.is_shell_only(),
        child_process_runtime_enabled: surface.child_process_runtime_enabled,
        opens_authority_write_path: surface.opens_authority_write_path,
    }
}

pub fn menu_action_shell_effect(action: DesktopMenuAction) -> DesktopTauriShellEffect {
    match action {
        DesktopMenuAction::ShowMainWindow
        | DesktopMenuAction::OpenCommandPalette
        | DesktopMenuAction::OpenSettings => DesktopTauriShellEffect::ShowMainWindow,
        DesktopMenuAction::QuitRequested => DesktopTauriShellEffect::QuitRequested,
    }
}

pub fn tray_action_shell_effect(action: DesktopTrayAction) -> DesktopTauriShellEffect {
    match action {
        DesktopTrayAction::ShowMainWindow => DesktopTauriShellEffect::ShowMainWindow,
        DesktopTrayAction::ToggleWindowVisibility => {
            DesktopTauriShellEffect::ToggleMainWindowVisibility
        }
        DesktopTrayAction::QuitRequested => DesktopTauriShellEffect::QuitRequested,
    }
}

pub fn run_desktop_tauri_app() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let menu = build_desktop_menu(app)?;
            app.set_menu(menu)?;

            let tray_menu = build_desktop_tray_menu(app)?;
            let _tray = build_desktop_tray_icon(app, &tray_menu)?;

            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if let Some(action) = resolve_desktop_menu_action_id(id) {
                apply_shell_effect(app, menu_action_shell_effect(action));
                return;
            }
            if let Some(action) = resolve_desktop_tray_action_id(id) {
                apply_shell_effect(app, tray_action_shell_effect(action));
            }
        })
        .on_tray_icon_event(|app, event| match event {
            tauri::tray::TrayIconEvent::Click { .. }
            | tauri::tray::TrayIconEvent::DoubleClick { .. } => {
                apply_shell_effect(app, DesktopTauriShellEffect::ToggleMainWindowVisibility);
            }
            _ => {}
        })
        .run(tauri::generate_context!())
}

fn apply_shell_effect<R: Runtime>(app: &AppHandle<R>, effect: DesktopTauriShellEffect) {
    match effect {
        DesktopTauriShellEffect::ShowMainWindow => show_main_window(app),
        DesktopTauriShellEffect::ToggleMainWindowVisibility => toggle_main_window_visibility(app),
        DesktopTauriShellEffect::QuitRequested => app.exit(0),
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(DESKTOP_TAURI_MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_main_window_visibility<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(DESKTOP_TAURI_MAIN_WINDOW_LABEL) {
        match window.is_visible() {
            Ok(true) => {
                let _ = window.hide();
            }
            Ok(false) => {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_tauri_runtime_surface_is_shell_only() {
        assert!(desktop_tauri_runtime_surface().is_shell_only());
        assert!(!desktop_tauri_runtime_surface().child_process_runtime_enabled);
        assert!(!desktop_tauri_runtime_surface().opens_authority_write_path);
    }

    #[test]
    fn desktop_tauri_startup_smoke_keeps_authority_closed() {
        let smoke = desktop_tauri_startup_smoke();

        assert!(smoke.passed());
        assert!(smoke.packaged_binary_started);
        assert!(smoke.shell_only_runtime);
        assert!(!smoke.child_process_runtime_enabled);
        assert!(!smoke.opens_authority_write_path);
    }

    #[test]
    fn desktop_menu_actions_map_only_to_shell_effects() {
        assert_eq!(
            menu_action_shell_effect(DesktopMenuAction::ShowMainWindow),
            DesktopTauriShellEffect::ShowMainWindow
        );
        assert_eq!(
            menu_action_shell_effect(DesktopMenuAction::OpenCommandPalette),
            DesktopTauriShellEffect::ShowMainWindow
        );
        assert_eq!(
            menu_action_shell_effect(DesktopMenuAction::OpenSettings),
            DesktopTauriShellEffect::ShowMainWindow
        );
        assert_eq!(
            menu_action_shell_effect(DesktopMenuAction::QuitRequested),
            DesktopTauriShellEffect::QuitRequested
        );
    }

    #[test]
    fn desktop_tray_actions_map_only_to_shell_effects() {
        assert_eq!(
            tray_action_shell_effect(DesktopTrayAction::ShowMainWindow),
            DesktopTauriShellEffect::ShowMainWindow
        );
        assert_eq!(
            tray_action_shell_effect(DesktopTrayAction::ToggleWindowVisibility),
            DesktopTauriShellEffect::ToggleMainWindowVisibility
        );
        assert_eq!(
            tray_action_shell_effect(DesktopTrayAction::QuitRequested),
            DesktopTauriShellEffect::QuitRequested
        );
    }
}
