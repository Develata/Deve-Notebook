//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!   - 15_settings#native-host-local-backend-preference
//!
//! Platform-owned RemoteBrowser -> LocalBackend recovery entrypoint. The
//! remote WebView has no callable IPC path into this module.

use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, WebviewWindow, Wry};

use crate::MobileNativeBackendState;

use self::coordinator::MobileBackendRecoveryCoordinator;
pub(crate) use self::state::{MobileBackendRecoverySnapshot, MobileBackendRecoveryState};

#[cfg(target_os = "android")]
mod android;
mod coordinator;
#[cfg(target_os = "ios")]
mod ios;
mod state;

type RecoveryCallback = Arc<dyn Fn() -> bool + Send + Sync>;

static RECOVERY_CALLBACK: OnceLock<Mutex<Option<RecoveryCallback>>> = OnceLock::new();

fn register_recovery_callback(callback: RecoveryCallback) -> Result<(), String> {
    let slot = RECOVERY_CALLBACK.get_or_init(|| Mutex::new(None));
    let mut slot = slot
        .lock()
        .map_err(|_| "mobile native recovery callback state poisoned".to_string())?;
    *slot = Some(callback);
    Ok(())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn invoke_registered_recovery() -> bool {
    RECOVERY_CALLBACK
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|callback| callback.clone())
        .is_some_and(|callback| callback())
}

pub(super) async fn install_mobile_backend_recovery_control(
    window: WebviewWindow<Wry>,
    app: AppHandle<Wry>,
    preference: Arc<MobileNativeBackendState>,
    recovery: Arc<MobileBackendRecoveryState>,
) -> Result<(), String> {
    let coordinator = MobileBackendRecoveryCoordinator::new(app, preference, recovery);
    register_recovery_callback(Arc::new(move || coordinator.request()))?;
    install_platform_recovery_control(&window).await
}

#[cfg(target_os = "android")]
async fn install_platform_recovery_control(window: &WebviewWindow<Wry>) -> Result<(), String> {
    android::install(window).await
}

#[cfg(target_os = "ios")]
async fn install_platform_recovery_control(window: &WebviewWindow<Wry>) -> Result<(), String> {
    ios::install(window).await
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
async fn install_platform_recovery_control(_window: &WebviewWindow<Wry>) -> Result<(), String> {
    Err("mobile native recovery control is unavailable on this target".to_string())
}

#[cfg(target_os = "android")]
async fn reset_platform_recovery_control(window: &WebviewWindow<Wry>) -> Result<(), String> {
    android::reset(window).await
}

#[cfg(target_os = "ios")]
async fn reset_platform_recovery_control(window: &WebviewWindow<Wry>) -> Result<(), String> {
    ios::reset(window).await
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
async fn reset_platform_recovery_control(_window: &WebviewWindow<Wry>) -> Result<(), String> {
    Err("mobile native recovery control is unavailable on this target".to_string())
}

#[cfg(target_os = "android")]
async fn remove_platform_recovery_control(window: &WebviewWindow<Wry>) -> Result<(), String> {
    android::remove(window).await
}

#[cfg(target_os = "ios")]
async fn remove_platform_recovery_control(window: &WebviewWindow<Wry>) -> Result<(), String> {
    ios::remove(window).await
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
async fn remove_platform_recovery_control(_window: &WebviewWindow<Wry>) -> Result<(), String> {
    Err("mobile native recovery control is unavailable on this target".to_string())
}
