//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-shell-modes
//!   - 15_settings#native-host-local-backend-preference
//!
//! Trusted LocalBackend command bridge and native-owned backend-mode transition.

use crate::{
    DesktopLocalServiceTauriState, DesktopNativeBackendState, probe_desktop_native_remote_backend,
};
use deve_core::native_adapter::{NativeBackendPreference, NativeBackendValidationResult};
use tauri::{AppHandle, Manager, State, WebviewWindow, Wry};

const UNTRUSTED_ORIGIN: &str = "native backend command requires bundled LocalBackend origin";

/// Owns the only process-lifecycle path that persists a Desktop backend mode.
/// Web callers can request a validated remote transition only while bundled
/// LocalBackend is active; native menu/tray callers can request local recovery.
struct DesktopBackendModeCoordinator {
    app: AppHandle<Wry>,
}

impl DesktopBackendModeCoordinator {
    fn new(app: AppHandle<Wry>) -> Self {
        Self { app }
    }

    fn transition_to_remote(
        &self,
        state: &DesktopNativeBackendState,
        origin: &str,
    ) -> Result<(), String> {
        if let Some(local_state) = self.app.try_state::<DesktopLocalServiceTauriState>()
            && let Err(error) = local_state.stop(super::current_unix_time_millis())
        {
            // The preference is still local, but stop may already have
            // retired the process handle. Restart to restore that old,
            // authoritative host preference instead of leaving a dead shell.
            self.app.request_restart();
            return Err(error.to_string());
        }
        if let Err(error) = state.save_preference(NativeBackendPreference::remote(origin)) {
            // The old preference is still local. Restart restores the stopped service.
            self.app.request_restart();
            return Err(error.to_string());
        }
        self.app.request_restart();
        Ok(())
    }

    fn transition_to_local(&self) -> Result<(), String> {
        let state = self
            .app
            .try_state::<DesktopNativeBackendState>()
            .ok_or_else(|| "desktop native backend state unavailable".to_string())?;
        state
            .save_preference(NativeBackendPreference::local())
            .map_err(|error| error.to_string())?;
        self.app.request_restart();
        Ok(())
    }
}

fn ensure_bundled_local_origin(window: &WebviewWindow<Wry>) -> Result<(), String> {
    let url = window.url().map_err(|error| error.to_string())?;
    if is_bundled_local_url(&url) {
        Ok(())
    } else {
        Err(UNTRUSTED_ORIGIN.to_string())
    }
}

fn is_bundled_local_url(url: &tauri::Url) -> bool {
    url.port().is_none()
        && matches!(
            (url.scheme(), url.host_str()),
            ("http", Some("tauri.localhost")) | ("tauri", Some("localhost"))
        )
}

#[tauri::command]
pub(super) async fn native_backend_get_config(
    window: WebviewWindow<Wry>,
    state: State<'_, DesktopNativeBackendState>,
) -> Result<NativeBackendPreference, String> {
    ensure_bundled_local_origin(&window)?;
    state.preference().map_err(|error| error.to_string())
}

#[tauri::command]
pub(super) async fn native_backend_validate_remote(
    window: WebviewWindow<Wry>,
    remote_url: String,
) -> Result<NativeBackendValidationResult, String> {
    ensure_bundled_local_origin(&window)?;
    Ok(probe_desktop_native_remote_backend(&remote_url).await)
}

#[tauri::command]
pub(super) async fn native_backend_save_remote(
    window: WebviewWindow<Wry>,
    app: AppHandle<Wry>,
    state: State<'_, DesktopNativeBackendState>,
    remote_url: String,
) -> Result<NativeBackendValidationResult, String> {
    ensure_bundled_local_origin(&window)?;
    let result = probe_desktop_native_remote_backend(&remote_url).await;
    if !result.ok {
        return Ok(result);
    }
    let origin = result
        .https_origin
        .as_deref()
        .ok_or_else(|| crate::DesktopNativeBackendError::InvalidNodeRolePayload.to_string())?;

    DesktopBackendModeCoordinator::new(app).transition_to_remote(&state, origin)?;
    Ok(result)
}

pub(super) fn switch_to_local_backend(app: &AppHandle<Wry>) {
    if let Err(error) = DesktopBackendModeCoordinator::new(app.clone()).transition_to_local() {
        eprintln!("desktop native backend local transition failed closed: {error}");
    }
}

pub(super) fn desktop_local_backend_command_plugin() -> tauri::plugin::TauriPlugin<Wry> {
    tauri::plugin::Builder::<Wry, ()>::new("deve-native-backend-commands")
        .invoke_handler(tauri::generate_handler![
            native_backend_get_config,
            native_backend_validate_remote,
            native_backend_save_remote,
        ])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_native_command_origins_are_exact() {
        for origin in ["http://tauri.localhost", "tauri://localhost"] {
            let url = tauri::Url::parse(origin).expect("trusted origin");
            assert!(is_bundled_local_url(&url));
        }
        for untrusted in [
            "https://remote.example",
            "http://tauri.localhost.evil.example",
            "http://tauri.localhost:4444",
        ] {
            let url = tauri::Url::parse(untrusted).expect("untrusted origin");
            assert!(!is_bundled_local_url(&url));
        }
    }
}
