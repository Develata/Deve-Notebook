//! plan_ref:
//!   - 17_tech_stack#native-packaging-dependency-gate
//!   - 11_ui_design/03_mobile#mobile-android-shell-package-execution-gate
//!   - 11_ui_design/03_mobile#mobile-ios-shell-package-execution-gate
//!
//! Mobile Tauri entrypoint.
//!
//! This module owns only the mobile shell mode boundary. LocalBackend is an
//! embedded loopback backend contract; RemoteBrowser only navigates to a remote
//! HTTPS origin. The shell does not write ledger, projection-workspace,
//! source-control, search, Git, or `.notegit` authority.

use deve_core::native_adapter::{
    NativeAdapterError, NativeBackendPreference, NativeRemoteTarget, NativeShellMode,
    native_shell_mode_for_backend_preference, validate_native_remote_target,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use thiserror::Error;

use crate::MobileNativeBackendState;
use crate::embedded_backend::{MobileEmbeddedBackendSupervisor, mobile_embedded_backend_plugin};
use crate::tauri_lifecycle::{handle_mobile_run_event, handle_mobile_window_event};

pub(crate) mod backend_recovery;
mod native_backend_commands;

use backend_recovery::{MobileBackendRecoveryState, install_mobile_backend_recovery_control};
use native_backend_commands::mobile_local_backend_command_plugin;

const MOBILE_TAURI_MAIN_WINDOW_LABEL: &str = "main";
pub const DEVE_NATIVE_REMOTE_URL_ENV: &str = "DEVE_NATIVE_REMOTE_URL";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileTauriRuntimeSurface {
    pub android_shell_package_entrypoint_declared: bool,
    pub ios_shell_package_entrypoint_declared: bool,
    pub build_script_declared: bool,
    pub webview_shell_runtime_declared: bool,
    pub local_backend_default_enabled: bool,
    pub embedded_service_runtime_enabled: bool,
    pub child_process_runtime_enabled: bool,
    pub opens_authority_write_path: bool,
    pub release_ready_claimed: bool,
}

impl MobileTauriRuntimeSurface {
    pub fn is_shell_only(self) -> bool {
        self.android_shell_package_entrypoint_declared
            && self.ios_shell_package_entrypoint_declared
            && self.build_script_declared
            && self.webview_shell_runtime_declared
            && self.local_backend_default_enabled
            && self.embedded_service_runtime_enabled
            && !self.child_process_runtime_enabled
            && !self.opens_authority_write_path
            && !self.release_ready_claimed
    }
}

pub fn mobile_tauri_runtime_surface() -> MobileTauriRuntimeSurface {
    MobileTauriRuntimeSurface {
        android_shell_package_entrypoint_declared: true,
        ios_shell_package_entrypoint_declared: true,
        build_script_declared: true,
        webview_shell_runtime_declared: true,
        local_backend_default_enabled: true,
        embedded_service_runtime_enabled: true,
        child_process_runtime_enabled: false,
        opens_authority_write_path: false,
        release_ready_claimed: false,
    }
}

#[derive(Debug, Error)]
pub enum MobileTauriModeError {
    #[error("--remote-url cannot be combined with an explicit LocalBackend mode")]
    ConflictingModes,
    #[error(transparent)]
    RemoteTarget(#[from] NativeAdapterError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MobileTauriModeResolution {
    remote_target: Option<NativeRemoteTarget>,
    native_local_recovery_control: bool,
}

const MOBILE_REMOTE_BROWSER_MODE_MARKER: &str =
    "deve_mobile native shell mode=RemoteBrowser embedded_backend=absent";

impl MobileTauriModeResolution {
    fn local() -> Self {
        Self {
            remote_target: None,
            native_local_recovery_control: false,
        }
    }

    fn remote_override(target: NativeRemoteTarget) -> Self {
        Self {
            remote_target: Some(target),
            native_local_recovery_control: false,
        }
    }

    fn remote_preference(target: NativeRemoteTarget) -> Self {
        Self {
            remote_target: Some(target),
            native_local_recovery_control: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MobileTauriLaunchOptions {
    pub remote_url: Option<String>,
    pub local_backend: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MobileTauriLaunchOptionsError {
    #[error("mobile remote browser URL argument requires a value")]
    MissingRemoteUrlValue,
    #[error("mobile remote browser URL must be an HTTPS origin")]
    InvalidRemoteUrl,
    #[error("mobile remote browser mode conflicts with forced local backend mode")]
    ConflictingModes,
}

impl MobileTauriLaunchOptions {
    pub fn from_args<I, S>(args: I) -> Result<Self, MobileTauriLaunchOptionsError>
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
                        .ok_or(MobileTauriLaunchOptionsError::MissingRemoteUrlValue)?;
                    options.remote_url = Some(value);
                }
                "--local-backend" => options.local_backend = Some(true),
                "--no-local-backend" => options.local_backend = Some(false),
                _ if arg.starts_with("--remote-url=") => {
                    let value = arg.trim_start_matches("--remote-url=");
                    if !is_non_flag_value(value) {
                        return Err(MobileTauriLaunchOptionsError::MissingRemoteUrlValue);
                    }
                    options.remote_url = Some(value.to_string());
                }
                _ if arg.starts_with("--remote=") => {
                    let value = arg.trim_start_matches("--remote=");
                    if !is_non_flag_value(value) {
                        return Err(MobileTauriLaunchOptionsError::MissingRemoteUrlValue);
                    }
                    options.remote_url = Some(value.to_string());
                }
                _ => {}
            }
        }

        if options.remote_url.is_some() && options.local_backend == Some(true) {
            return Err(MobileTauriLaunchOptionsError::ConflictingModes);
        }
        if let Some(remote_url) = options.remote_url.as_deref() {
            validate_native_remote_target(&NativeRemoteTarget {
                https_origin: remote_url.to_string(),
            })
            .map_err(|_| MobileTauriLaunchOptionsError::InvalidRemoteUrl)?;
        }
        Ok(options)
    }
}

fn mobile_tauri_remote_browser_target_from_env()
-> Result<Option<NativeRemoteTarget>, MobileTauriModeError> {
    let Some(value) = std::env::var_os(DEVE_NATIVE_REMOTE_URL_ENV) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    let target = NativeRemoteTarget {
        https_origin: value.to_string_lossy().into_owned(),
    };
    validate_native_remote_target(&target)?;
    Ok(Some(target))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_mobile_tauri_app() {
    let options = match MobileTauriLaunchOptions::from_args(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("deve_mobile launch options failed closed: {error}");
            return;
        }
    };
    run_mobile_tauri_app_with_launch_options(options)
}

pub fn run_mobile_tauri_app_with_launch_options(options: MobileTauriLaunchOptions) {
    let mut builder = tauri::Builder::default();

    builder = builder
        .on_window_event(handle_mobile_window_event)
        .setup(move |app| {
            let app_data_dir_result = app.path().app_data_dir().map_err(|error| error.to_string());
            let host_backend_preference = load_host_backend_preference(&app_data_dir_result);
            let mode =
                match mobile_tauri_mode_for_launch_options(&options, &host_backend_preference) {
                    Ok(mode) => mode,
                    Err(error) => {
                        eprintln!("deve_mobile RemoteBrowser config failed closed: {error}");
                        return Ok(());
                    }
                };
            let native_backend_state = std::sync::Arc::new(
                MobileNativeBackendState::from_data_root(app_data_dir_result.clone()),
            );
            app.manage(native_backend_state.clone());
            let backend_recovery_state =
                std::sync::Arc::new(MobileBackendRecoveryState::default());
            app.manage(backend_recovery_state.clone());

            if mode.remote_target.is_some() {
                eprintln!("{MOBILE_REMOTE_BROWSER_MODE_MARKER}");
            }

            if mode.remote_target.is_none() && options.local_backend != Some(false) {
                let app_data_dir = match app_data_dir_result {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("deve_mobile LocalBackend app data dir failed closed: {error}");
                        return Ok(());
                    }
                };
                match MobileEmbeddedBackendSupervisor::start(app_data_dir) {
                    Ok((supervisor, bootstrap)) => {
                        if let Err(error) =
                            app.handle().plugin(mobile_local_backend_command_plugin())
                        {
                            eprintln!(
                                "deve_mobile LocalBackend command plugin failed closed: {error}"
                            );
                            return Ok(());
                        }
                        if let Err(error) = app
                            .handle()
                            .plugin(mobile_embedded_backend_plugin(&bootstrap.script))
                        {
                            eprintln!("deve_mobile LocalBackend plugin failed closed: {error}");
                            return Ok(());
                        }
                        app.manage(std::sync::Arc::new(supervisor));
                    }
                    Err(error) => {
                        eprintln!("deve_mobile LocalBackend bootstrap failed closed: {error}");
                        return Ok(());
                    }
                }
            }
            let window = match create_mobile_main_window(app, mode.remote_target.as_ref()) {
                Ok(window) => window,
                Err(error) => {
                    eprintln!("deve_mobile main WebView creation failed closed: {error}");
                    return Ok(());
                }
            };
            if mode.native_local_recovery_control {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = install_mobile_backend_recovery_control(
                        window,
                        app_handle,
                        native_backend_state,
                        backend_recovery_state,
                    )
                    .await
                    {
                        eprintln!(
                            "deve_mobile native LocalBackend recovery control failed closed: {error}"
                        );
                    }
                });
            }
            Ok(())
        });

    let app = match builder.build(tauri::generate_context!()) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("deve_mobile Tauri shell build failed closed: {error}");
            return;
        }
    };
    app.run(handle_mobile_run_event);
}

fn mobile_tauri_mode_for_launch_options(
    options: &MobileTauriLaunchOptions,
    host_backend_preference: &NativeBackendPreference,
) -> Result<MobileTauriModeResolution, MobileTauriModeError> {
    let environment_target = mobile_tauri_remote_browser_target_from_env()?;
    mobile_tauri_mode_for_inputs(options, host_backend_preference, environment_target)
}

fn mobile_tauri_mode_for_inputs(
    options: &MobileTauriLaunchOptions,
    host_backend_preference: &NativeBackendPreference,
    environment_target: Option<NativeRemoteTarget>,
) -> Result<MobileTauriModeResolution, MobileTauriModeError> {
    if options.remote_url.is_some() && options.local_backend.is_some() {
        return Err(MobileTauriModeError::ConflictingModes);
    }
    if let Some(remote_url) = options.remote_url.as_deref() {
        let target = NativeRemoteTarget {
            https_origin: remote_url.to_string(),
        };
        validate_native_remote_target(&target)?;
        return Ok(MobileTauriModeResolution::remote_override(target));
    }
    if options.local_backend == Some(true) {
        return Ok(MobileTauriModeResolution::local());
    }
    if let Some(target) = environment_target {
        return Ok(MobileTauriModeResolution::remote_override(target));
    }
    if options.local_backend == Some(false) {
        return Ok(MobileTauriModeResolution::local());
    }
    match native_shell_mode_for_backend_preference(host_backend_preference) {
        Ok(NativeShellMode::RemoteBrowser { target }) => {
            validate_native_remote_target(&target)?;
            Ok(MobileTauriModeResolution::remote_preference(target))
        }
        Ok(NativeShellMode::LocalBackend) => Ok(MobileTauriModeResolution::local()),
        Err(error) => Err(MobileTauriModeError::RemoteTarget(error)),
    }
}

fn load_host_backend_preference(
    app_data_dir_result: &Result<std::path::PathBuf, String>,
) -> NativeBackendPreference {
    let Some(app_data_dir) = app_data_dir_result.as_ref().ok() else {
        return NativeBackendPreference::local();
    };
    match crate::load_mobile_native_backend_preference(app_data_dir) {
        Ok(preference) => preference,
        Err(error) => {
            eprintln!("mobile native backend preference ignored: {error}");
            NativeBackendPreference::local()
        }
    }
}

fn is_non_flag_value(value: &str) -> bool {
    !value.trim().is_empty() && !value.starts_with("--")
}

fn create_mobile_main_window<R: tauri::Runtime>(
    app: &impl Manager<R>,
    remote_target: Option<&NativeRemoteTarget>,
) -> Result<tauri::WebviewWindow<R>, String> {
    if let Some(window) = app.get_webview_window(MOBILE_TAURI_MAIN_WINDOW_LABEL) {
        return Ok(window);
    }
    let Some(mut window_config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MOBILE_TAURI_MAIN_WINDOW_LABEL)
        .cloned()
    else {
        return Err("deve_mobile Tauri config is missing main window".to_string());
    };
    if let Some(target) = remote_target {
        let url = tauri::Url::parse(&target.https_origin)
            .map_err(|_| "deve_mobile RemoteBrowser URL parse failed closed".to_string())?;
        window_config.url = WebviewUrl::External(url);
    }
    WebviewWindowBuilder::from_config(app, &window_config)
        .and_then(|builder| builder.build())
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "android")]
fn create_mobile_main_window_from_android_activity(
    app: &impl Manager<tauri::Wry>,
    activity_name: &str,
    created_by_activity_name: &str,
) -> Result<tauri::WebviewWindow<tauri::Wry>, String> {
    if app
        .get_webview_window(MOBILE_TAURI_MAIN_WINDOW_LABEL)
        .is_some()
    {
        return Err("deve_mobile stale main window blocked Android recovery".to_string());
    }
    let Some(window_config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MOBILE_TAURI_MAIN_WINDOW_LABEL)
    else {
        return Err("deve_mobile Tauri config is missing main window".to_string());
    };
    WebviewWindowBuilder::from_config(app, window_config)
        .map_err(|_| "deve_mobile Android local main window config failed".to_string())?
        .activity_name(activity_name)
        .created_by_activity_name(created_by_activity_name)
        .build()
        .map_err(|_| "deve_mobile Android local main Activity creation failed".to_string())
}

#[cfg(test)]
mod tests;
