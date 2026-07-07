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
    DesktopLocalServiceTauriState, DesktopMenuAction, DesktopNativeBackendState, DesktopTrayAction,
    build_desktop_menu, build_desktop_tray_icon, build_desktop_tray_menu,
    desktop_tauri_bootstrap_plugin, desktop_tauri_local_service_bootstrap_from_env,
    desktop_tauri_local_service_bootstrap_with_policy,
    desktop_tauri_remote_browser_bootstrap_from_env,
    desktop_tauri_remote_browser_bootstrap_from_origin, resolve_desktop_local_service_data_root,
    resolve_desktop_menu_action_id, resolve_desktop_tray_action_id,
};
use deve_core::native_adapter::{
    NativeBackendPreference, NativeBackendValidationResult, NativeShellMode,
    native_shell_mode_for_backend_preference,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, RunEvent, Runtime, State, WindowEvent, Wry};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopRunEventShutdownAction {
    None,
    StopLocalBackend,
    StopLocalBackendAndExit,
}

impl DesktopRunEventShutdownAction {
    fn should_stop_local_backend(self) -> bool {
        matches!(self, Self::StopLocalBackend | Self::StopLocalBackendAndExit)
    }

    fn should_exit_process(self) -> bool {
        matches!(self, Self::StopLocalBackendAndExit)
    }
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
    let native_backend_data_root =
        resolve_desktop_local_service_data_root().map_err(|error| error.to_string());
    let host_backend_preference = load_host_backend_preference(&native_backend_data_root);
    let remote_browser_bootstrap =
        match remote_browser_bootstrap_for_launch_options(&options, &host_backend_preference) {
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
    let native_backend_state = DesktopNativeBackendState::from_data_root(native_backend_data_root);
    let mut builder = tauri::Builder::default();

    if let Some(script) = remote_browser_bootstrap.as_ref() {
        builder = builder.plugin(desktop_tauri_bootstrap_plugin(script));
    } else if let Some(bootstrap) = local_service_bootstrap.as_ref() {
        builder = builder.plugin(desktop_tauri_bootstrap_plugin(&bootstrap.script));
    }

    builder
        .invoke_handler(tauri::generate_handler![
            native_backend_get_config,
            native_backend_validate_remote,
            native_backend_save_remote,
            native_backend_switch_local,
        ])
        .setup(move |app| {
            app.manage(native_backend_state);
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
        .build(tauri::generate_context!())?
        .run(handle_desktop_run_event::<Wry>);
    Ok(())
}

fn remote_browser_bootstrap_for_launch_options(
    options: &DesktopTauriLaunchOptions,
    host_backend_preference: &NativeBackendPreference,
) -> Result<Option<crate::DesktopTauriBootstrapScript>, crate::DesktopTauriBootstrapError> {
    if let Some(remote_url) = options.remote_url.as_deref() {
        return desktop_tauri_remote_browser_bootstrap_from_origin(remote_url).map(Some);
    }
    if options.local_backend == Some(true) {
        return Ok(None);
    }
    if let Some(bootstrap) = desktop_tauri_remote_browser_bootstrap_from_env()? {
        return Ok(Some(bootstrap));
    }
    if options.local_backend == Some(false) {
        return Ok(None);
    }
    match native_shell_mode_for_backend_preference(host_backend_preference) {
        Ok(NativeShellMode::RemoteBrowser { target }) => {
            desktop_tauri_remote_browser_bootstrap_from_origin(&target.https_origin).map(Some)
        }
        Ok(NativeShellMode::LocalBackend) => Ok(None),
        Err(error) => Err(crate::DesktopTauriBootstrapError::RemoteTarget(error)),
    }
}

fn load_host_backend_preference(
    native_backend_data_root: &Result<std::path::PathBuf, String>,
) -> NativeBackendPreference {
    let Some(data_root) = native_backend_data_root.as_ref().ok() else {
        return NativeBackendPreference::local();
    };
    match crate::load_desktop_native_backend_preference(data_root) {
        Ok(preference) => preference,
        Err(error) => {
            eprintln!("desktop native backend preference ignored: {error}");
            NativeBackendPreference::local()
        }
    }
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

#[tauri::command]
async fn native_backend_get_config(
    state: State<'_, DesktopNativeBackendState>,
) -> Result<NativeBackendPreference, String> {
    state.preference().map_err(|error| error.to_string())
}

#[tauri::command]
async fn native_backend_validate_remote(remote_url: String) -> NativeBackendValidationResult {
    crate::probe_desktop_native_remote_backend(&remote_url).await
}

#[tauri::command]
async fn native_backend_save_remote(
    app: AppHandle<Wry>,
    state: State<'_, DesktopNativeBackendState>,
    remote_url: String,
) -> Result<NativeBackendValidationResult, String> {
    let result = crate::probe_desktop_native_remote_backend(&remote_url).await;
    if !result.ok {
        return Ok(result);
    }
    let origin = result
        .https_origin
        .as_deref()
        .ok_or_else(|| crate::DesktopNativeBackendError::InvalidNodeRolePayload.to_string())?;
    state
        .save_preference(NativeBackendPreference::remote(origin))
        .map_err(|error| error.to_string())?;
    if let Some(local_state) = app.try_state::<crate::DesktopLocalServiceTauriState>() {
        local_state
            .stop(current_unix_time_millis())
            .map_err(|error| error.to_string())?;
    }
    if let Some(window) = app.get_webview_window(DESKTOP_TAURI_MAIN_WINDOW_LABEL) {
        let url = tauri::Url::parse(origin)
            .map_err(|error| crate::DesktopNativeBackendError::NavigationFailed(error.to_string()))
            .map_err(|error| error.to_string())?;
        window
            .navigate(url)
            .map_err(|error| crate::DesktopNativeBackendError::NavigationFailed(error.to_string()))
            .map_err(|error| error.to_string())?;
    }
    Ok(result)
}

#[tauri::command]
async fn native_backend_switch_local(
    app: AppHandle<Wry>,
    state: State<'_, DesktopNativeBackendState>,
) -> Result<NativeBackendPreference, String> {
    let preference = NativeBackendPreference::local();
    state
        .save_preference(preference.clone())
        .map_err(|error| error.to_string())?;
    app.request_restart();
    Ok(preference)
}

fn apply_shell_effect<R: Runtime>(app: &AppHandle<R>, effect: DesktopTauriShellEffect) {
    match effect {
        DesktopTauriShellEffect::ShowMainWindow => show_main_window(app),
        DesktopTauriShellEffect::ToggleMainWindowVisibility => toggle_main_window_visibility(app),
        DesktopTauriShellEffect::QuitRequested => app.exit(0),
    }
}

fn handle_desktop_run_event<R: Runtime>(app: &AppHandle<R>, event: RunEvent) {
    match event {
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            let action = desktop_shutdown_action_for_window_close(&label);
            if action.should_exit_process() {
                api.prevent_close();
            }
            apply_desktop_shutdown_action(app, action, current_unix_time_millis());
        }
        RunEvent::ExitRequested { .. } | RunEvent::Exit => {
            apply_desktop_shutdown_action(
                app,
                desktop_shutdown_action_for_process_exit(),
                current_unix_time_millis(),
            );
        }
        _ => {}
    }
}

fn desktop_main_window_close_exits_process(label: &str) -> bool {
    label == DESKTOP_TAURI_MAIN_WINDOW_LABEL
}

fn desktop_shutdown_action_for_window_close(label: &str) -> DesktopRunEventShutdownAction {
    if desktop_main_window_close_exits_process(label) {
        DesktopRunEventShutdownAction::StopLocalBackendAndExit
    } else {
        DesktopRunEventShutdownAction::None
    }
}

fn desktop_shutdown_action_for_process_exit() -> DesktopRunEventShutdownAction {
    DesktopRunEventShutdownAction::StopLocalBackend
}

fn apply_desktop_shutdown_action<R: Runtime>(
    app: &AppHandle<R>,
    action: DesktopRunEventShutdownAction,
    timestamp_unix_ms: i64,
) {
    if action.should_stop_local_backend() {
        stop_local_backend_for_app(app, timestamp_unix_ms);
    }
    if action.should_exit_process() {
        app.exit(0);
    }
}

fn stop_local_backend_for_app<R: Runtime>(app: &AppHandle<R>, timestamp_unix_ms: i64) {
    if let Some(local_state) = app.try_state::<crate::DesktopLocalServiceTauriState>()
        && let Err(error) = local_state.stop(timestamp_unix_ms)
    {
        eprintln!("desktop local service stop failed during app shutdown: {error}");
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
mod tests;
