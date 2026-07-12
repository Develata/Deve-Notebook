//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!
//! Tauri lifecycle projection for the Mobile embedded-backend supervisor.

use std::sync::Arc;

use tauri::{Manager, RunEvent, Window, WindowEvent};

use crate::embedded_backend::{
    MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT, MobileEmbeddedBackendServiceState,
    MobileEmbeddedBackendSupervisor,
};

#[cfg(mobile)]
const NATIVE_SUSPENDED_EVENT: &str = "deve-native-suspended";
#[cfg(mobile)]
const NATIVE_RESUMED_EVENT: &str = "deve-native-resumed";
#[cfg(mobile)]
const NATIVE_SERVICE_ERROR_EVENT: &str = "deve-native-service-error";

pub(crate) fn handle_mobile_window_event<R: tauri::Runtime>(
    window: &Window<R>,
    event: &WindowEvent,
) {
    #[cfg(mobile)]
    match event {
        WindowEvent::Suspended => {
            let Some(state) = window
                .app_handle()
                .try_state::<Arc<MobileEmbeddedBackendSupervisor>>()
            else {
                return;
            };
            match state.suspend() {
                Ok(transition_token) => {
                    dispatch_window_event_with_transition(
                        window,
                        NATIVE_SUSPENDED_EVENT,
                        transition_token,
                    );
                }
                Err(error) => {
                    eprintln!("deve_mobile LocalBackend suspend failed closed: {error}");
                    dispatch_window_event(window, NATIVE_SERVICE_ERROR_EVENT);
                }
            }
        }
        WindowEvent::Resumed => {
            let Some(state) = window
                .app_handle()
                .try_state::<Arc<MobileEmbeddedBackendSupervisor>>()
            else {
                return;
            };
            let supervisor = Arc::clone(&state);
            let app = window.app_handle().clone();
            let label = window.label().to_string();
            tauri::async_runtime::spawn(async move {
                let Some(webview) = app.get_webview_window(&label) else {
                    let error = supervisor
                        .record_resume_webview_unavailable()
                        .expect_err("missing WebView fails closed");
                    eprintln!("deve_mobile LocalBackend resume failed closed: {error}");
                    return;
                };
                match supervisor
                    .resume_and_complete_on_webview(
                        &webview,
                        NATIVE_RESUMED_EVENT,
                        NATIVE_SERVICE_ERROR_EVENT,
                    )
                    .await
                {
                    Ok(()) => {}
                    Err(crate::embedded_backend::MobileEmbeddedBackendError::LifecycleTransitionCancelled) => {
                        eprintln!("deve_mobile stale LocalBackend resume ignored");
                    }
                    Err(error) => {
                        eprintln!("deve_mobile LocalBackend resume failed closed: {error}");
                    }
                }
            });
        }
        _ => {}
    }

    #[cfg(not(mobile))]
    let _ = (window, event);
}

pub(crate) fn handle_mobile_run_event<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    event: RunEvent,
) {
    let RunEvent::ExitRequested { code, api, .. } = event else {
        return;
    };
    let Some(state) = app.try_state::<Arc<MobileEmbeddedBackendSupervisor>>() else {
        return;
    };

    if let Ok(snapshot) = state.snapshot() {
        if snapshot.service_state == MobileEmbeddedBackendServiceState::Stopped {
            return;
        }
        api.prevent_exit();
        if snapshot.service_state == MobileEmbeddedBackendServiceState::Stopping {
            return;
        }
    } else {
        api.prevent_exit();
    }

    let supervisor = Arc::clone(&state);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = supervisor
            .shutdown(MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT)
            .await
        {
            eprintln!("deve_mobile LocalBackend exit shutdown failed closed: {error}");
        } else {
            eprintln!("deve_mobile LocalBackend clean shutdown complete");
        }
        app.exit(code.unwrap_or(0));
    });
}

pub(crate) async fn shutdown_mobile_backend_before_restart<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let Some(state) = app.try_state::<Arc<MobileEmbeddedBackendSupervisor>>() else {
        return Ok(());
    };
    let supervisor = Arc::clone(&state);
    supervisor
        .shutdown(MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT)
        .await
        .map_err(|error| error.to_string())
}

#[cfg(mobile)]
fn dispatch_window_event<R: tauri::Runtime>(window: &Window<R>, event: &str) {
    if let Some(webview) = window.app_handle().get_webview_window(window.label()) {
        dispatch_webview_event(&webview, event);
    }
}

#[cfg(mobile)]
fn dispatch_window_event_with_transition<R: tauri::Runtime>(
    window: &Window<R>,
    event: &str,
    transition_token: u64,
) {
    if let Some(webview) = window.app_handle().get_webview_window(window.label())
        && let Err(error) = webview.eval(guarded_lifecycle_event_source(transition_token, event))
    {
        eprintln!("deve_mobile lifecycle event dispatch failed closed: {error}");
    }
}

#[cfg(mobile)]
fn guarded_lifecycle_event_source(transition_token: u64, event: &str) -> String {
    format!(
        "(()=>{{const k='__DEVE_NATIVE_LIFECYCLE_TRANSITION__';const n={transition_token};const c=Number(window[k]??0);if(c<n){{window[k]=n;window.dispatchEvent(new Event({event:?}));}}}})();"
    )
}

#[cfg(mobile)]
fn dispatch_webview_event<R: tauri::Runtime>(webview: &tauri::WebviewWindow<R>, event: &str) {
    if let Err(error) = webview.eval(format!("window.dispatchEvent(new Event({event:?}));")) {
        eprintln!("deve_mobile lifecycle event dispatch failed closed: {error}");
    }
}
