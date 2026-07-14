//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Android platform-owned backend recovery control.

use tauri::{WebviewWindow, Wry};

use super::invoke_registered_recovery;

const PLATFORM_CONTROL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

pub(super) async fn install(window: &WebviewWindow<Wry>) -> Result<(), String> {
    invoke_control_method(window, "installUseLocalBackendControl").await
}

pub(super) async fn reset(window: &WebviewWindow<Wry>) -> Result<(), String> {
    invoke_control_method(window, "resetUseLocalBackendControl").await
}

pub(super) async fn remove(window: &WebviewWindow<Wry>) -> Result<(), String> {
    invoke_control_method(window, "removeUseLocalBackendControl").await
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
