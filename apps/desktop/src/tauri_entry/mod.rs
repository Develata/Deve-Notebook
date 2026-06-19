//! plan_ref:
//!   - 17_tech_stack#native-packaging-dependency-gate
//!   - 11_ui_design/02_desktop#desktop-packaging-scaffold
//!
//! Desktop Tauri window-shell entrypoint.
//!
//! The default native-packaging runtime starts the LocalBackend shell path.
//! LocalBackend must inject either a session-bound endpoint bootstrap or a
//! recovery bootstrap before Web startup. RemoteBrowser only navigates to a
//! validated HTTPS origin.
//! This module does not write ledger, projection-workspace, source-control, search, Git, or
//! `.notegit` authority.

use crate::{
    DESKTOP_TAURI_MAIN_WINDOW_LABEL, DesktopLocalServiceTauriState, DesktopMenuAction,
    DesktopTrayAction, build_desktop_menu, build_desktop_tray_icon, build_desktop_tray_menu,
    desktop_tauri_bootstrap_plugin, desktop_tauri_local_service_bootstrap_from_env,
    desktop_tauri_remote_browser_bootstrap_from_env, resolve_desktop_menu_action_id,
    resolve_desktop_tray_action_id,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

mod smoke;

pub use smoke::{
    DESKTOP_TAURI_NATIVE_SESSION_SMOKE_OK, DESKTOP_TAURI_STARTUP_SMOKE_OK,
    DesktopTauriNativeSessionSmoke, DesktopTauriRuntimeSurface, DesktopTauriStartupSmoke,
    desktop_tauri_native_session_smoke, desktop_tauri_runtime_surface, desktop_tauri_startup_smoke,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopTauriShellEffect {
    ShowMainWindow,
    ToggleMainWindowVisibility,
    QuitRequested,
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
    let remote_browser_bootstrap = match desktop_tauri_remote_browser_bootstrap_from_env() {
        Ok(bootstrap) => bootstrap,
        Err(error) => {
            eprintln!("desktop remote browser bootstrap refused: {error}");
            return Ok(());
        }
    };
    let mut local_service_bootstrap = if remote_browser_bootstrap.is_none() {
        desktop_tauri_local_service_bootstrap_from_env(current_unix_time_millis())
    } else {
        None
    };
    let mut service_runtime = local_service_bootstrap
        .as_mut()
        .and_then(|bootstrap| bootstrap.runtime.take());
    let mut builder = tauri::Builder::default();

    if let Some(script) = remote_browser_bootstrap.as_ref() {
        builder = builder.plugin(desktop_tauri_bootstrap_plugin(script));
    } else if let Some(bootstrap) = local_service_bootstrap.as_ref() {
        builder = builder.plugin(desktop_tauri_bootstrap_plugin(&bootstrap.script));
    }

    builder
        .setup(move |app| {
            if let Some(runtime) = service_runtime.take() {
                app.manage(DesktopLocalServiceTauriState::new(runtime));
            }

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

fn current_unix_time_millis() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
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
    use crate::DEVE_DESKTOP_LOCAL_SERVICE_ENV;

    #[test]
    fn desktop_tauri_runtime_surface_is_shell_only() {
        assert!(desktop_tauri_runtime_surface().is_shell_only());
        assert!(desktop_tauri_runtime_surface().local_backend_default_enabled);
        assert!(desktop_tauri_runtime_surface().child_process_runtime_enabled);
        assert!(!desktop_tauri_runtime_surface().opens_authority_write_path);
    }

    #[test]
    fn desktop_tauri_startup_smoke_keeps_authority_closed() {
        let smoke = desktop_tauri_startup_smoke();

        assert!(smoke.passed());
        assert!(smoke.packaged_binary_started);
        assert!(smoke.shell_only_runtime);
        assert!(smoke.local_backend_default_enabled);
        assert!(smoke.child_process_runtime_enabled);
        assert!(!smoke.opens_authority_write_path);
    }

    #[test]
    fn desktop_tauri_native_session_smoke_reports_disabled_when_local_backend_disabled() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let _guard = EnvGuard::set(DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("0"));
        let smoke = desktop_tauri_native_session_smoke(1).expect("smoke");

        assert!(!smoke.passed());
        assert!(!smoke.local_service_started);
        assert!(!smoke.session_bound);
        assert!(!smoke.native_session_cookie_installed_before_bootstrap);
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

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        old: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let old = std::env::var(key).ok();
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            Self { key, old }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match self.old.as_ref() {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
}
