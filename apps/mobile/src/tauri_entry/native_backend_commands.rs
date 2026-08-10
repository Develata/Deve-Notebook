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

use super::backend_recovery::{PlatformColdRestartSource, request_platform_cold_restart};

const MOBILE_MAIN_WINDOW_LABEL: &str = "main";
const UNTRUSTED_ORIGIN: &str = "native backend command requires bundled LocalBackend origin";
#[cfg(any(mobile, test))]
const INITIAL_NATIVE_SESSION_HANDOFF_FAILURE_CATEGORIES: &[&str] = &[
    "android_native_cookie_callback_rejected",
    "android_native_cookie_not_retained",
    "android_native_cookie_verification_failed",
    "android_native_cookie_callback_invalid",
    "android_native_cookie_jni_setup_failed",
    "android_native_cookie_callback_already_pending",
    "android_native_cookie_request_id_exhausted",
    "android_native_cookie_callback_channel_closed",
    "android_native_cookie_callback_timeout",
    "android_native_cookie_callback_registry_poisoned",
    "android_native_cookie_webview_dispatch_failed",
];

#[cfg(any(mobile, test))]
fn initial_native_session_handoff_failure_category(
    error: &crate::embedded_backend::MobileEmbeddedBackendError,
) -> &'static str {
    let crate::embedded_backend::MobileEmbeddedBackendError::WebviewInstallFailed(source) = error
    else {
        return "native_session_handoff_failed";
    };
    INITIAL_NATIVE_SESSION_HANDOFF_FAILURE_CATEGORIES
        .iter()
        .copied()
        .find(|category| *category == source.as_str())
        .unwrap_or("native_session_handoff_failed")
}

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
        let result = state.prepare_initial_webview_session(&webview).await;
        if let Err(error) = &result {
            let category = initial_native_session_handoff_failure_category(error);
            eprintln!("deve_mobile initial native session handoff failed closed: {category}");
        }
        result.map_err(|_| "mobile WebView session preparation failed".to_string())
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
        request_platform_cold_restart(&app, PlatformColdRestartSource::Main).await;
        return Err(error.to_string());
    }
    request_platform_cold_restart(&app, PlatformColdRestartSource::Main).await;
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
    use super::{initial_native_session_handoff_failure_category, is_bundled_local_url};
    use crate::embedded_backend::MobileEmbeddedBackendError;

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

    #[test]
    fn android_native_session_handoff_failure_logs_fixed_category_without_secret() {
        let secret = "secret-sentinel-must-not-reach-diagnostics";
        assert_eq!(
            initial_native_session_handoff_failure_category(
                &MobileEmbeddedBackendError::WebviewInstallFailed(
                    "android_native_cookie_callback_timeout".to_string()
                )
            ),
            "android_native_cookie_callback_timeout"
        );
        let unknown = initial_native_session_handoff_failure_category(
            &MobileEmbeddedBackendError::WebviewInstallFailed(secret.to_string()),
        );
        assert_eq!(unknown, "native_session_handoff_failed");
        assert!(!unknown.contains(secret));
    }
}
