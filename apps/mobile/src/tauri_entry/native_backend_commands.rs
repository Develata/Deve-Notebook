//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!   - 15_settings#native-host-local-backend-preference
//!
//! LocalBackend-only Mobile IPC surface. RemoteBrowser never installs this plugin.

use crate::embedded_backend::MobileEmbeddedBackendSupervisor;
use crate::tauri_entry::backend_recovery::MobileBackendRecoveryState;
use crate::tauri_lifecycle::shutdown_mobile_backend_before_restart;
use crate::{MobileNativeBackendState, probe_mobile_native_remote_backend};
use deve_core::native_adapter::{NativeBackendPreference, NativeBackendValidationResult};
use tauri::{AppHandle, Manager, State, WebviewWindow, Wry};

const MOBILE_MAIN_WINDOW_LABEL: &str = "main";
const UNTRUSTED_ORIGIN: &str = "native backend command requires bundled LocalBackend origin";

fn is_bundled_local_url(url: &tauri::Url) -> bool {
    url.port().is_none()
        && matches!(
            (url.scheme(), url.host_str()),
            ("http", Some("tauri.localhost")) | ("tauri", Some("localhost"))
        )
}

fn ensure_bundled_local_origin(window: &WebviewWindow<Wry>) -> Result<(), String> {
    let url = window.url().map_err(|error| error.to_string())?;
    is_bundled_local_url(&url)
        .then_some(())
        .ok_or_else(|| UNTRUSTED_ORIGIN.to_string())
}

#[tauri::command]
async fn native_backend_get_config(
    window: WebviewWindow<Wry>,
    state: State<'_, std::sync::Arc<MobileNativeBackendState>>,
) -> Result<NativeBackendPreference, String> {
    ensure_bundled_local_origin(&window)?;
    state.preference().map_err(|error| error.to_string())
}

#[tauri::command]
async fn native_backend_get_service_state(
    window: WebviewWindow<Wry>,
    app: AppHandle<Wry>,
) -> Result<Option<crate::MobileEmbeddedBackendSupervisorSnapshot>, String> {
    ensure_bundled_local_origin(&window)?;
    let Some(state) = app.try_state::<std::sync::Arc<MobileEmbeddedBackendSupervisor>>() else {
        return Ok(None);
    };
    state
        .snapshot()
        .map(Some)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn native_backend_get_recovery_state(
    window: WebviewWindow<Wry>,
    app: AppHandle<Wry>,
) -> Result<crate::tauri_entry::backend_recovery::MobileBackendRecoverySnapshot, String> {
    ensure_bundled_local_origin(&window)?;
    let state = app
        .try_state::<std::sync::Arc<MobileBackendRecoveryState>>()
        .ok_or_else(|| "mobile backend recovery state unavailable".to_string())?;
    state.snapshot()
}

#[tauri::command]
async fn native_backend_prepare_webview_session(
    window: WebviewWindow<Wry>,
    app: AppHandle<Wry>,
) -> Result<(), String> {
    ensure_bundled_local_origin(&window)?;
    let state = app
        .try_state::<std::sync::Arc<MobileEmbeddedBackendSupervisor>>()
        .ok_or_else(|| "mobile embedded runtime unavailable".to_string())?;
    let webview = app
        .get_webview_window(MOBILE_MAIN_WINDOW_LABEL)
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
async fn native_backend_debug_stop_transport(
    window: WebviewWindow<Wry>,
    app: AppHandle<Wry>,
) -> Result<(), String> {
    ensure_bundled_local_origin(&window)?;
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
async fn native_backend_debug_request_exit(
    window: WebviewWindow<Wry>,
    app: AppHandle<Wry>,
) -> Result<(), String> {
    ensure_bundled_local_origin(&window)?;
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
async fn native_backend_validate_remote(
    window: WebviewWindow<Wry>,
    remote_url: String,
) -> Result<NativeBackendValidationResult, String> {
    ensure_bundled_local_origin(&window)?;
    Ok(probe_mobile_native_remote_backend(&remote_url).await)
}

#[tauri::command]
async fn native_backend_save_remote(
    window: WebviewWindow<Wry>,
    app: AppHandle<Wry>,
    state: State<'_, std::sync::Arc<MobileNativeBackendState>>,
    remote_url: String,
) -> Result<NativeBackendValidationResult, String> {
    ensure_bundled_local_origin(&window)?;
    let result = probe_mobile_native_remote_backend(&remote_url).await;
    if !result.ok {
        return Ok(result);
    }
    let origin = result
        .https_origin
        .as_deref()
        .ok_or_else(|| crate::MobileNativeBackendError::InvalidNodeRolePayload.to_string())?;

    shutdown_mobile_backend_before_restart(&app).await?;
    if let Err(error) = state.save_preference(NativeBackendPreference::remote(origin)) {
        // The persisted preference is still local; restart restores the retired runtime.
        app.request_restart();
        return Err(error.to_string());
    }
    app.request_restart();
    Ok(result)
}

pub(super) fn mobile_local_backend_command_plugin() -> tauri::plugin::TauriPlugin<Wry> {
    tauri::plugin::Builder::<Wry, ()>::new("deve-native-backend-commands")
        .invoke_handler(tauri::generate_handler![
            native_backend_get_config,
            native_backend_get_service_state,
            native_backend_get_recovery_state,
            native_backend_prepare_webview_session,
            native_backend_debug_stop_transport,
            native_backend_debug_request_exit,
            native_backend_validate_remote,
            native_backend_save_remote,
        ])
        .build()
}

#[cfg(test)]
mod tests {
    use super::is_bundled_local_url;

    #[test]
    fn mobile_local_commands_accept_only_bundled_origins() {
        for origin in ["http://tauri.localhost", "tauri://localhost"] {
            assert!(is_bundled_local_url(
                &tauri::Url::parse(origin).expect("trusted origin")
            ));
        }
        for origin in ["https://remote.example", "http://tauri.localhost:4444"] {
            assert!(!is_bundled_local_url(
                &tauri::Url::parse(origin).expect("remote origin")
            ));
        }
    }
}
