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
    NativeAdapterError, NativeBackendPreference, NativeBackendValidationResult, NativeRemoteTarget,
    NativeShellMode, native_shell_mode_for_backend_preference, validate_native_remote_target,
};
use tauri::{AppHandle, Manager, State, WebviewWindowBuilder, Wry};
use thiserror::Error;

use crate::MobileNativeBackendState;
use crate::embedded_backend::{MobileEmbeddedBackendSupervisor, mobile_embedded_backend_plugin};
use crate::tauri_lifecycle::{
    handle_mobile_run_event, handle_mobile_window_event, shutdown_mobile_backend_before_restart,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileTauriRemoteBrowserScript {
    source: String,
}

impl MobileTauriRemoteBrowserScript {
    pub fn source(&self) -> &str {
        &self.source
    }
}

#[derive(Debug, Error)]
pub enum MobileTauriModeError {
    #[error(transparent)]
    RemoteTarget(#[from] NativeAdapterError),
    #[error("mobile RemoteBrowser source contains forbidden material: {marker}")]
    ForbiddenMaterial { marker: &'static str },
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
            mobile_tauri_remote_browser_init_script(&NativeRemoteTarget {
                https_origin: remote_url.to_string(),
            })
            .map_err(|_| MobileTauriLaunchOptionsError::InvalidRemoteUrl)?;
        }
        Ok(options)
    }
}

pub fn mobile_tauri_remote_browser_init_script(
    target: &NativeRemoteTarget,
) -> Result<MobileTauriRemoteBrowserScript, MobileTauriModeError> {
    validate_native_remote_target(target)?;
    let origin = serde_json::to_string(&target.https_origin)
        .expect("serializing a validated HTTPS origin string cannot fail");
    let source = format!(
        "(()=>{{const target=new URL({origin}).origin;if(window.top===window&&window.location.origin!==target){{window.location.replace(target);}}}})();"
    );
    validate_mobile_remote_script_source(&source)?;
    Ok(MobileTauriRemoteBrowserScript { source })
}

fn mobile_tauri_remote_browser_script_from_env()
-> Result<Option<MobileTauriRemoteBrowserScript>, MobileTauriModeError> {
    let Some(value) = std::env::var_os(DEVE_NATIVE_REMOTE_URL_ENV) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    let target = NativeRemoteTarget {
        https_origin: value.to_string_lossy().into_owned(),
    };
    mobile_tauri_remote_browser_init_script(&target).map(Some)
}

fn validate_mobile_remote_script_source(source: &str) -> Result<(), MobileTauriModeError> {
    let source_lower = source.to_ascii_lowercase();
    for marker in [
        "<script",
        "</script",
        "token",
        "secret",
        "localstorage",
        "location.href",
    ] {
        if source_lower.contains(marker) {
            return Err(MobileTauriModeError::ForbiddenMaterial { marker });
        }
    }
    Ok(())
}

fn mobile_tauri_remote_browser_plugin<R: tauri::Runtime>(
    script: MobileTauriRemoteBrowserScript,
) -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R, ()>::new("deve-mobile-remote-browser")
        .js_init_script(script.source)
        .build()
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
        .invoke_handler(tauri::generate_handler![
            native_backend_get_config,
            native_backend_get_service_state,
            native_backend_prepare_webview_session,
            native_backend_debug_stop_transport,
            native_backend_debug_request_exit,
            native_backend_validate_remote,
            native_backend_save_remote,
            native_backend_switch_local,
        ])
        .setup(move |app| {
            let app_data_dir_result = app.path().app_data_dir().map_err(|error| error.to_string());
            let host_backend_preference = load_host_backend_preference(&app_data_dir_result);
            let remote_browser_script = match remote_browser_script_for_launch_options(
                &options,
                &host_backend_preference,
            ) {
                Ok(script) => script,
                Err(error) => {
                    eprintln!("deve_mobile RemoteBrowser config failed closed: {error}");
                    return Ok(());
                }
            };
            app.manage(MobileNativeBackendState::from_data_root(
                app_data_dir_result.clone(),
            ));

            if let Some(script) = remote_browser_script {
                if let Err(error) = app
                    .handle()
                    .plugin(mobile_tauri_remote_browser_plugin(script))
                {
                    eprintln!("deve_mobile RemoteBrowser plugin failed closed: {error}");
                    return Ok(());
                }
            } else if options.local_backend != Some(false) {
                let app_data_dir = match app_data_dir_result {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("deve_mobile LocalBackend app data dir failed closed: {error}");
                        return Ok(());
                    }
                };
                match MobileEmbeddedBackendSupervisor::start(app_data_dir) {
                    Ok((supervisor, bootstrap)) => {
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
            create_mobile_main_window(app);
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

fn remote_browser_script_for_launch_options(
    options: &MobileTauriLaunchOptions,
    host_backend_preference: &NativeBackendPreference,
) -> Result<Option<MobileTauriRemoteBrowserScript>, MobileTauriModeError> {
    if let Some(remote_url) = options.remote_url.as_deref() {
        return mobile_tauri_remote_browser_init_script(&NativeRemoteTarget {
            https_origin: remote_url.to_string(),
        })
        .map(Some);
    }
    if options.local_backend == Some(true) {
        return Ok(None);
    }
    if let Some(script) = mobile_tauri_remote_browser_script_from_env()? {
        return Ok(Some(script));
    }
    if options.local_backend == Some(false) {
        return Ok(None);
    }
    match native_shell_mode_for_backend_preference(host_backend_preference) {
        Ok(NativeShellMode::RemoteBrowser { target }) => {
            mobile_tauri_remote_browser_init_script(&target).map(Some)
        }
        Ok(NativeShellMode::LocalBackend) => Ok(None),
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

#[tauri::command]
async fn native_backend_get_config(
    state: State<'_, MobileNativeBackendState>,
) -> Result<NativeBackendPreference, String> {
    state.preference().map_err(|error| error.to_string())
}

#[tauri::command]
async fn native_backend_get_service_state(
    app: AppHandle<Wry>,
) -> Result<Option<crate::MobileEmbeddedBackendSupervisorSnapshot>, String> {
    let Some(state) = app.try_state::<std::sync::Arc<MobileEmbeddedBackendSupervisor>>() else {
        return Ok(None);
    };
    state
        .snapshot()
        .map(Some)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn native_backend_prepare_webview_session(app: AppHandle<Wry>) -> Result<(), String> {
    let state = app
        .try_state::<std::sync::Arc<MobileEmbeddedBackendSupervisor>>()
        .ok_or_else(|| "mobile embedded runtime unavailable".to_string())?;
    let webview = app
        .get_webview_window(MOBILE_TAURI_MAIN_WINDOW_LABEL)
        .ok_or_else(|| "mobile main WebView unavailable".to_string())?;
    #[cfg(mobile)]
    {
        state
            .prepare_initial_webview_session(&webview)
            .await
            .map_err(|error| error.to_string())
    }
    #[cfg(not(mobile))]
    {
        let _ = (state, webview);
        Err("mobile WebView session preparation is unavailable on this target".to_string())
    }
}

#[tauri::command]
async fn native_backend_debug_stop_transport(app: AppHandle<Wry>) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        let state = app
            .try_state::<std::sync::Arc<MobileEmbeddedBackendSupervisor>>()
            .ok_or_else(|| "mobile embedded runtime unavailable".to_string())?;
        state
            .stop_transport_for_lifecycle_smoke()
            .map_err(|error| error.to_string())
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = app;
        Err("mobile lifecycle fault injection is debug-only".to_string())
    }
}

#[tauri::command]
async fn native_backend_debug_request_exit(app: AppHandle<Wry>) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        app.exit(0);
        Ok(())
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = app;
        Err("mobile lifecycle exit probe is debug-only".to_string())
    }
}

#[tauri::command]
async fn native_backend_validate_remote(remote_url: String) -> NativeBackendValidationResult {
    crate::probe_mobile_native_remote_backend(&remote_url).await
}

#[tauri::command]
async fn native_backend_save_remote(
    app: AppHandle<Wry>,
    state: State<'_, MobileNativeBackendState>,
    remote_url: String,
) -> Result<NativeBackendValidationResult, String> {
    let result = crate::probe_mobile_native_remote_backend(&remote_url).await;
    if !result.ok {
        return Ok(result);
    }
    let origin = result
        .https_origin
        .as_deref()
        .ok_or_else(|| crate::MobileNativeBackendError::InvalidNodeRolePayload.to_string())?;
    state
        .save_preference(NativeBackendPreference::remote(origin))
        .map_err(|error| error.to_string())?;
    shutdown_mobile_backend_before_restart(&app).await?;
    app.request_restart();
    Ok(result)
}

#[tauri::command]
async fn native_backend_switch_local(
    app: AppHandle<Wry>,
    state: State<'_, MobileNativeBackendState>,
) -> Result<NativeBackendPreference, String> {
    let preference = NativeBackendPreference::local();
    state
        .save_preference(preference.clone())
        .map_err(|error| error.to_string())?;
    shutdown_mobile_backend_before_restart(&app).await?;
    app.request_restart();
    Ok(preference)
}

fn create_mobile_main_window<R: tauri::Runtime>(app: &tauri::App<R>) {
    if app
        .get_webview_window(MOBILE_TAURI_MAIN_WINDOW_LABEL)
        .is_some()
    {
        return;
    }
    let Some(window_config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MOBILE_TAURI_MAIN_WINDOW_LABEL)
    else {
        eprintln!("deve_mobile Tauri config is missing main window");
        return;
    };
    if let Err(error) =
        WebviewWindowBuilder::from_config(app, window_config).and_then(|builder| builder.build())
    {
        eprintln!("deve_mobile main WebView creation failed closed: {error}");
    }
}

#[cfg(test)]
mod tests;
