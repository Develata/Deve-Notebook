//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!
//! Bounded recovery-window and candidate-runtime cleanup primitives.

use std::sync::Arc;

use tauri::{AppHandle, Manager, Wry};

use crate::embedded_backend::{
    MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT, MobileEmbeddedBackendSupervisor,
};

use super::{
    PlatformRecoveryAnchor, platform_recovery_anchor_label, retire_platform_recovery_anchor,
};

const WINDOW_RETIRE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub(super) async fn shutdown_candidate(supervisor: &MobileEmbeddedBackendSupervisor) -> bool {
    match supervisor
        .shutdown(MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT)
        .await
    {
        Ok(()) => true,
        Err(error) => {
            eprintln!("deve_mobile candidate LocalBackend shutdown failed closed: {error}");
            false
        }
    }
}

pub(super) async fn shutdown_managed_supervisor(app: &AppHandle<Wry>) -> bool {
    let Some(supervisor) = app
        .try_state::<Arc<MobileEmbeddedBackendSupervisor>>()
        .map(|state| state.inner().clone())
    else {
        eprintln!(
            "deve_mobile managed LocalBackend supervisor unavailable during recovery cleanup"
        );
        return false;
    };
    shutdown_candidate(&supervisor).await
}

pub(super) async fn retire_and_confirm_recovery_anchor(
    app: &AppHandle<Wry>,
    recovery_anchor: &PlatformRecoveryAnchor,
) -> bool {
    let dispatch_failed = retire_platform_recovery_anchor(recovery_anchor)
        .await
        .is_err();
    let retired = match platform_recovery_anchor_label() {
        Some(label) => wait_for_window_retirement(app, label).await,
        None => !dispatch_failed,
    };
    if dispatch_failed && retired {
        eprintln!(
            "deve_mobile recovery anchor retirement committed despite an unconfirmed dispatch"
        );
    }
    retired
}

pub(super) async fn wait_for_window_retirement(app: &AppHandle<Wry>, label: &str) -> bool {
    let deadline = tokio::time::Instant::now() + WINDOW_RETIRE_TIMEOUT;
    loop {
        if app.get_webview_window(label).is_none() {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
