//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Android platform-owned backend recovery control and lifecycle anchor.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry};

use super::super::create_mobile_main_window_from_android_activity;
use super::invoke_registered_recovery;

const PLATFORM_CONTROL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const PLATFORM_ACTIVITY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
pub(super) const ANDROID_RECOVERY_ANCHOR_LABEL: &str = "deve-mobile-recovery-anchor";
const ANDROID_RECOVERY_ANCHOR_ACTIVITY: &str = "RecoveryAnchorActivity";

pub(super) struct RecoveryAnchor {
    window: WebviewWindow<Wry>,
    main_activity_name: String,
}

pub(super) async fn install(window: &WebviewWindow<Wry>) -> Result<(), String> {
    invoke_control_method(window, "installUseLocalBackendControl").await
}

pub(super) async fn reset(window: &WebviewWindow<Wry>) -> Result<(), String> {
    invoke_control_method(window, "resetUseLocalBackendControl").await
}

pub(super) async fn remove(window: &WebviewWindow<Wry>) -> Result<(), String> {
    invoke_control_method(window, "removeUseLocalBackendControl").await
}

pub(super) fn create_recovery_anchor(
    app: &AppHandle<Wry>,
    remote_window: &WebviewWindow<Wry>,
) -> Result<RecoveryAnchor, String> {
    if app
        .get_webview_window(ANDROID_RECOVERY_ANCHOR_LABEL)
        .is_some()
    {
        return Err("android_recovery_anchor_already_exists".to_string());
    }
    let main_activity_name = remote_window
        .activity_name()
        .map_err(|_| "android_remote_activity_name_unavailable".to_string())?;
    let url = tauri::Url::parse("about:blank")
        .map_err(|_| "android_recovery_anchor_url_invalid".to_string())?;
    let window = WebviewWindowBuilder::new(
        app,
        ANDROID_RECOVERY_ANCHOR_LABEL,
        WebviewUrl::External(url),
    )
    .activity_name(ANDROID_RECOVERY_ANCHOR_ACTIVITY)
    .created_by_activity_name(&main_activity_name)
    .visible(false)
    .build()
    .map_err(|_| "android_recovery_anchor_create_failed".to_string())?;
    Ok(RecoveryAnchor {
        window,
        main_activity_name,
    })
}

pub(super) async fn retire_remote_activity(window: &WebviewWindow<Wry>) -> Result<(), String> {
    finish_activity(window).await
}

pub(super) fn create_local_main_window(
    app: &AppHandle<Wry>,
    anchor: &RecoveryAnchor,
) -> Result<WebviewWindow<Wry>, String> {
    create_mobile_main_window_from_android_activity(
        app,
        &anchor.main_activity_name,
        ANDROID_RECOVERY_ANCHOR_ACTIVITY,
    )
}

pub(super) async fn retire_recovery_anchor(anchor: &RecoveryAnchor) -> Result<(), String> {
    finish_activity(&anchor.window).await
}

pub(super) async fn schedule_cold_restart(
    app: &AppHandle<Wry>,
    source_label: &str,
) -> Result<(), String> {
    let window = app
        .get_webview_window(source_label)
        .ok_or_else(|| "android_cold_restart_activity_unavailable".to_string())?;
    invoke_control_method(&window, "scheduleBackendRecoveryColdStart").await
}

pub(super) fn retire_process(exit_code: i32) -> ! {
    // This is Android self-retirement after a separate helper process has
    // accepted the launcher-task handoff; it is not a child-process runtime.
    std::process::exit(exit_code)
}

async fn finish_activity(window: &WebviewWindow<Wry>) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |platform| {
            platform
                .jni_handle()
                .exec(move |env, activity, _android_webview| {
                    let result = match env.call_method(activity, "finish", "()V", &[]) {
                        Ok(_) => Ok(()),
                        Err(jni::errors::Error::JavaException) => {
                            let _ = env.exception_clear();
                            Err("android_activity_finish_java_exception".to_string())
                        }
                        Err(_) => Err("android_activity_finish_jni_failed".to_string()),
                    };
                    let _ = sender.send(result);
                });
        })
        .map_err(|_| "android_activity_finish_dispatch_failed".to_string())?;
    tokio::time::timeout(PLATFORM_ACTIVITY_TIMEOUT, receiver)
        .await
        .map_err(|_| "android_activity_finish_timeout".to_string())?
        .map_err(|_| "android_activity_finish_result_channel_closed".to_string())?
}

async fn invoke_control_method(
    window: &WebviewWindow<Wry>,
    method: &'static str,
) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |platform| {
            platform
                .jni_handle()
                .exec(move |env, activity, _android_webview| {
                    let result = env
                        .call_method(activity, method, "()Z", &[])
                        .and_then(|value| value.z())
                        .map_err(|error| error.to_string())
                        .and_then(|installed| {
                            installed.then_some(()).ok_or_else(|| {
                                format!("Android native method {method} returned false")
                            })
                        });
                    let _ = sender.send(result);
                });
        })
        .map_err(|error| error.to_string())?;
    tokio::time::timeout(PLATFORM_CONTROL_TIMEOUT, receiver)
        .await
        .map_err(|_| format!("Android native method {method} timed out"))?
        .map_err(|_| format!("Android native method {method} result channel closed"))?
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_deve_notebook_mobile_MainActivity_requestUseLocalBackend(
    _env: jni::JNIEnv<'_>,
    _activity: jni::objects::JObject<'_>,
) -> jni::sys::jboolean {
    if invoke_registered_recovery() { 1 } else { 0 }
}
