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
    DESKTOP_TAURI_MAIN_WINDOW_LABEL, DesktopLocalServiceEntrypointPolicy,
    DesktopLocalServiceTauriState, DesktopMenuAction, DesktopTrayAction, build_desktop_menu,
    build_desktop_tray_icon, build_desktop_tray_menu, desktop_tauri_bootstrap_plugin,
    desktop_tauri_local_service_bootstrap_from_env,
    desktop_tauri_local_service_bootstrap_with_policy,
    desktop_tauri_remote_browser_bootstrap_from_env,
    desktop_tauri_remote_browser_bootstrap_from_origin, resolve_desktop_menu_action_id,
    resolve_desktop_tray_action_id,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};
use thiserror::Error;

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopTauriLaunchOptions {
    pub remote_url: Option<String>,
    pub local_backend: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DesktopTauriLaunchOptionsError {
    #[error("desktop remote browser URL argument requires a value")]
    MissingRemoteUrlValue,
    #[error("desktop remote browser URL must be an HTTPS origin")]
    InvalidRemoteUrl,
    #[error("desktop remote browser mode conflicts with forced local backend mode")]
    ConflictingModes,
}

impl DesktopTauriLaunchOptions {
    pub fn from_args<I, S>(args: I) -> Result<Self, DesktopTauriLaunchOptionsError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut options = Self::default();
        let mut iter = args.into_iter().map(Into::into);

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--remote-url" | "--remote" => {
                    let value = iter
                        .next()
                        .filter(|value| is_non_flag_value(value))
                        .ok_or(DesktopTauriLaunchOptionsError::MissingRemoteUrlValue)?;
                    options.remote_url = Some(value);
                }
                "--local-backend" => options.local_backend = Some(true),
                "--no-local-backend" => options.local_backend = Some(false),
                _ if arg.starts_with("--remote-url=") => {
                    let value = arg.trim_start_matches("--remote-url=");
                    if !is_non_flag_value(value) {
                        return Err(DesktopTauriLaunchOptionsError::MissingRemoteUrlValue);
                    }
                    options.remote_url = Some(value.to_string());
                }
                _ if arg.starts_with("--remote=") => {
                    let value = arg.trim_start_matches("--remote=");
                    if !is_non_flag_value(value) {
                        return Err(DesktopTauriLaunchOptionsError::MissingRemoteUrlValue);
                    }
                    options.remote_url = Some(value.to_string());
                }
                _ => {}
            }
        }

        if options.remote_url.is_some() && options.local_backend == Some(true) {
            return Err(DesktopTauriLaunchOptionsError::ConflictingModes);
        }
        if let Some(remote_url) = options.remote_url.as_deref() {
            desktop_tauri_remote_browser_bootstrap_from_origin(remote_url)
                .map_err(|_| DesktopTauriLaunchOptionsError::InvalidRemoteUrl)?;
        }
        Ok(options)
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
    run_desktop_tauri_app_with_launch_options(DesktopTauriLaunchOptions::default())
}

pub fn run_desktop_tauri_app_with_launch_options(
    options: DesktopTauriLaunchOptions,
) -> tauri::Result<()> {
    let remote_browser_bootstrap = match remote_browser_bootstrap_for_launch_options(&options) {
        Ok(bootstrap) => bootstrap,
        Err(error) => {
            eprintln!("desktop remote browser bootstrap refused: {error}");
            return Ok(());
        }
    };
    let timestamp_unix_ms = current_unix_time_millis();
    let mut local_service_bootstrap = if remote_browser_bootstrap.is_none() {
        match options.local_backend {
            Some(true) => desktop_tauri_local_service_bootstrap_with_policy(
                timestamp_unix_ms,
                DesktopLocalServiceEntrypointPolicy::local_backend_default(),
            ),
            Some(false) => None,
            None => desktop_tauri_local_service_bootstrap_from_env(timestamp_unix_ms),
        }
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

fn remote_browser_bootstrap_for_launch_options(
    options: &DesktopTauriLaunchOptions,
) -> Result<Option<crate::DesktopTauriBootstrapScript>, crate::DesktopTauriBootstrapError> {
    if let Some(remote_url) = options.remote_url.as_deref() {
        return desktop_tauri_remote_browser_bootstrap_from_origin(remote_url).map(Some);
    }
    desktop_tauri_remote_browser_bootstrap_from_env()
}

fn is_non_flag_value(value: &str) -> bool {
    !value.trim().is_empty() && !value.starts_with("--")
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
    fn desktop_launch_options_parse_remote_browser_url() {
        let options =
            DesktopTauriLaunchOptions::from_args(["--remote-url", "https://deve.example"])
                .expect("options");

        assert_eq!(options.remote_url.as_deref(), Some("https://deve.example"));
        assert_eq!(options.local_backend, None);
    }

    #[test]
    fn desktop_launch_options_parse_remote_browser_url_equals_form() {
        let options = DesktopTauriLaunchOptions::from_args(["--remote-url=https://deve.example"])
            .expect("options");

        assert_eq!(options.remote_url.as_deref(), Some("https://deve.example"));
        assert_eq!(options.local_backend, None);
    }

    #[test]
    fn desktop_launch_options_reject_conflicting_local_and_remote_modes() {
        let error = DesktopTauriLaunchOptions::from_args([
            "--remote-url",
            "https://deve.example",
            "--local-backend",
        ])
        .expect_err("conflicting mode must fail");

        assert_eq!(error, DesktopTauriLaunchOptionsError::ConflictingModes);
    }

    #[test]
    fn desktop_launch_options_reject_missing_remote_url_value() {
        let error = DesktopTauriLaunchOptions::from_args(["--remote-url", "--local-backend"])
            .expect_err("missing url must fail");

        assert_eq!(error, DesktopTauriLaunchOptionsError::MissingRemoteUrlValue);
    }

    #[test]
    fn desktop_launch_options_reject_invalid_remote_browser_url() {
        let error = DesktopTauriLaunchOptions::from_args(["--remote-url", "http://deve.example"])
            .expect_err("invalid url must fail");

        assert_eq!(error, DesktopTauriLaunchOptionsError::InvalidRemoteUrl);
    }

    #[test]
    fn desktop_launch_options_support_manual_local_backend_disable() {
        let options =
            DesktopTauriLaunchOptions::from_args(["--no-local-backend"]).expect("options");

        assert_eq!(options.remote_url, None);
        assert_eq!(options.local_backend, Some(false));
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
