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
mod cleanup;
mod coordinator;
#[cfg(target_os = "ios")]
mod ios;
mod state;

type RecoveryCallback = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Clone, Copy)]
pub(super) enum PlatformColdRestartSource {
    Main,
    RecoveryAnchor,
}

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

#[cfg(target_os = "android")]
type PlatformRecoveryAnchor = android::RecoveryAnchor;

#[cfg(not(target_os = "android"))]
struct PlatformRecoveryAnchor;

#[cfg(target_os = "android")]
fn create_platform_recovery_anchor(
    app: &AppHandle<Wry>,
    remote_window: &WebviewWindow<Wry>,
) -> Result<PlatformRecoveryAnchor, String> {
    android::create_recovery_anchor(app, remote_window)
}

#[cfg(not(target_os = "android"))]
fn create_platform_recovery_anchor(
    _app: &AppHandle<Wry>,
    _remote_window: &WebviewWindow<Wry>,
) -> Result<PlatformRecoveryAnchor, String> {
    Ok(PlatformRecoveryAnchor)
}

#[cfg(target_os = "android")]
async fn retire_platform_remote_surface(window: &WebviewWindow<Wry>) -> Result<(), String> {
    android::retire_remote_activity(window).await
}

#[cfg(not(target_os = "android"))]
async fn retire_platform_remote_surface(window: &WebviewWindow<Wry>) -> Result<(), String> {
    window.destroy().map_err(|error| error.to_string())
}

#[cfg(target_os = "android")]
fn create_platform_local_main_window(
    app: &AppHandle<Wry>,
    anchor: &PlatformRecoveryAnchor,
) -> Result<WebviewWindow<Wry>, String> {
    android::create_local_main_window(app, anchor)
}

#[cfg(not(target_os = "android"))]
fn create_platform_local_main_window(
    app: &AppHandle<Wry>,
    _anchor: &PlatformRecoveryAnchor,
) -> Result<WebviewWindow<Wry>, String> {
    super::create_mobile_main_window(app, None)
}

#[cfg(target_os = "android")]
async fn retire_platform_recovery_anchor(anchor: &PlatformRecoveryAnchor) -> Result<(), String> {
    android::retire_recovery_anchor(anchor).await
}

#[cfg(not(target_os = "android"))]
async fn retire_platform_recovery_anchor(_anchor: &PlatformRecoveryAnchor) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
fn platform_recovery_anchor_label() -> Option<&'static str> {
    Some(android::ANDROID_RECOVERY_ANCHOR_LABEL)
}

#[cfg(not(target_os = "android"))]
fn platform_recovery_anchor_label() -> Option<&'static str> {
    None
}

pub(super) async fn request_platform_cold_restart(
    app: &AppHandle<Wry>,
    source: PlatformColdRestartSource,
) {
    #[cfg(target_os = "android")]
    {
        let label = match source {
            PlatformColdRestartSource::Main => super::MOBILE_TAURI_MAIN_WINDOW_LABEL,
            PlatformColdRestartSource::RecoveryAnchor => android::ANDROID_RECOVERY_ANCHOR_LABEL,
        };
        let exit_code = match android::schedule_cold_restart(app, label).await {
            Ok(()) => 0,
            Err(_) => {
                eprintln!("deve_mobile Android cold restart scheduling failed closed");
                1
            }
        };
        // The normal Tauri ExitRequested gate intentionally waits for the
        // managed supervisor. A recovery-required process must retire even
        // when that supervisor could not stop; the separate helper process
        // already owns the bounded launcher-task handoff.
        android::retire_process(exit_code);
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = source;
        app.request_restart();
    }
}
