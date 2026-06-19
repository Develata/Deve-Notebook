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
    NativeAdapterError, NativeRemoteTarget, validate_native_remote_target,
};
use tauri::{Manager, WebviewWindowBuilder};
use thiserror::Error;

use crate::embedded_backend::{
    mobile_embedded_backend_plugin, run_mobile_embedded_backend_bootstrap_with_port_retry,
};

const MOBILE_TAURI_MAIN_WINDOW_LABEL: &str = "main";

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

pub fn mobile_tauri_remote_browser_init_script(
    target: &NativeRemoteTarget,
) -> Result<MobileTauriRemoteBrowserScript, MobileTauriModeError> {
    validate_native_remote_target(target)?;
    let origin = serde_json::to_string(&target.https_origin)
        .expect("serializing a validated HTTPS origin string cannot fail");
    let source = format!("window.location.replace({origin});");
    validate_mobile_remote_script_source(&source)?;
    Ok(MobileTauriRemoteBrowserScript { source })
}

fn mobile_tauri_remote_browser_script_from_env()
-> Result<Option<MobileTauriRemoteBrowserScript>, MobileTauriModeError> {
    let Some(value) = std::env::var_os("DEVE_NATIVE_REMOTE_URL") else {
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
    let mut builder = tauri::Builder::default();
    let remote_browser_script = match mobile_tauri_remote_browser_script_from_env() {
        Ok(script) => script,
        Err(error) => {
            eprintln!("deve_mobile RemoteBrowser config failed closed: {error}");
            return;
        }
    };
    let start_local_backend = remote_browser_script.is_none();
    if let Some(script) = remote_browser_script {
        builder = builder.plugin(mobile_tauri_remote_browser_plugin(script));
    }

    builder = builder.setup(move |app| {
        if start_local_backend {
            let app_data_dir = match app.path().app_data_dir() {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("deve_mobile LocalBackend app data dir failed closed: {error}");
                    return Ok(());
                }
            };
            match run_mobile_embedded_backend_bootstrap_with_port_retry(app_data_dir) {
                Ok(bootstrap) => {
                    if let Err(error) = app
                        .handle()
                        .plugin(mobile_embedded_backend_plugin(&bootstrap.script))
                    {
                        eprintln!("deve_mobile LocalBackend plugin failed closed: {error}");
                        return Ok(());
                    }
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

    let result = builder.run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("deve_mobile Tauri shell exited with error: {error}");
    }
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
mod tests {
    use super::*;

    #[test]
    fn mobile_tauri_runtime_surface_is_shell_only() {
        let surface = mobile_tauri_runtime_surface();

        assert!(surface.is_shell_only());
        assert!(surface.android_shell_package_entrypoint_declared);
        assert!(surface.ios_shell_package_entrypoint_declared);
        assert!(surface.build_script_declared);
        assert!(surface.webview_shell_runtime_declared);
        assert!(surface.local_backend_default_enabled);
        assert!(surface.embedded_service_runtime_enabled);
        assert!(!surface.child_process_runtime_enabled);
        assert!(!surface.opens_authority_write_path);
        assert!(!surface.release_ready_claimed);
    }

    #[test]
    fn mobile_tauri_remote_browser_accepts_https_origin_without_native_bootstrap() {
        let script = mobile_tauri_remote_browser_init_script(&NativeRemoteTarget {
            https_origin: "https://deve.example".to_string(),
        })
        .expect("remote script");

        assert!(script.source().contains("window.location.replace"));
        assert!(script.source().contains("https://deve.example"));
        assert!(!script.source().contains("__DEVE_NATIVE_BOOTSTRAP"));
        assert!(!script.source().contains("http_base"));
        assert!(!script.source().contains("ws_base"));
    }

    #[test]
    fn mobile_tauri_remote_browser_rejects_non_https_origin() {
        let error = mobile_tauri_remote_browser_init_script(&NativeRemoteTarget {
            https_origin: "http://deve.example".to_string(),
        })
        .expect_err("http remote target must fail");

        assert!(matches!(
            error,
            MobileTauriModeError::RemoteTarget(NativeAdapterError::WrongScheme {
                expected_scheme: "https",
                ..
            })
        ));
    }

    #[test]
    fn mobile_tauri_main_window_creation_is_deferred_until_bootstrap() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri config");
        assert_eq!(
            config
                .pointer("/app/windows/0/label")
                .and_then(|value| value.as_str()),
            Some(MOBILE_TAURI_MAIN_WINDOW_LABEL)
        );
        assert_eq!(
            config
                .pointer("/app/windows/0/create")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
    }
}
