//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!   - 15_settings#native-host-local-backend-preference
//!
//! Failure-atomic native backend recovery coordinator.

use std::sync::Arc;

use deve_core::native_adapter::{NativeBackendMode, NativeBackendPreference};
use tauri::{AppHandle, Manager, Wry};
use thiserror::Error;

use crate::MobileNativeBackendState;
use crate::embedded_backend::{
    MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT, MobileEmbeddedBackendSupervisor,
    mobile_embedded_backend_plugin,
};

use super::super::{
    MOBILE_TAURI_MAIN_WINDOW_LABEL, create_mobile_main_window, mobile_local_backend_command_plugin,
};
use super::state::{MobileBackendRecoveryPhase, MobileBackendRecoveryState};
use super::{remove_platform_recovery_control, reset_platform_recovery_control};

const REMOTE_WINDOW_RETIRE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Error)]
enum MobileBackendRecoveryError {
    #[error("mobile LocalBackend recovery is only available from a remote preference")]
    PreferenceNotRemote,
    #[error("mobile LocalBackend recovery app data dir unavailable: {0}")]
    AppDataDirUnavailable(String),
    #[error("mobile LocalBackend recovery supervisor start task failed: {0}")]
    SupervisorStartTask(String),
    #[error("mobile LocalBackend recovery supervisor start failed: {0}")]
    SupervisorStart(String),
    #[error("mobile LocalBackend recovery preference write failed: {0}")]
    PreferenceWrite(String),
    #[error("mobile RemoteBrowser main WebView is unavailable")]
    RemoteWindowUnavailable,
    #[error("mobile native recovery control retirement failed: {0}")]
    NativeControlRetire(String),
    #[error("mobile RemoteBrowser WebView destruction failed: {0}")]
    RemoteWindowDestroy(String),
    #[error("mobile RemoteBrowser WebView did not retire before the timeout")]
    RemoteWindowRetireTimeout,
    #[error("mobile LocalBackend command plugin registration failed: {0}")]
    CommandPlugin(String),
    #[error("mobile LocalBackend bootstrap plugin registration failed: {0}")]
    BootstrapPlugin(String),
    #[error("mobile LocalBackend supervisor state was already managed")]
    SupervisorAlreadyManaged,
    #[error("mobile LocalBackend WebView creation failed: {0}")]
    LocalWindowCreate(String),
    #[error("mobile LocalBackend recovery transition state failed: {0}")]
    TransitionState(String),
}

struct RecoveryFailure {
    error: MobileBackendRecoveryError,
    restart_required: bool,
    active_runtime_owners: u8,
}

impl RecoveryFailure {
    fn remote_active(error: MobileBackendRecoveryError) -> Self {
        Self {
            error,
            restart_required: false,
            active_runtime_owners: 0,
        }
    }

    fn after_retirement(error: MobileBackendRecoveryError, active_runtime_owners: u8) -> Self {
        Self {
            error,
            restart_required: true,
            active_runtime_owners,
        }
    }
}

#[derive(Clone)]
pub(super) struct MobileBackendRecoveryCoordinator {
    app: AppHandle<Wry>,
    preference: Arc<MobileNativeBackendState>,
    recovery: Arc<MobileBackendRecoveryState>,
}

impl MobileBackendRecoveryCoordinator {
    pub(super) fn new(
        app: AppHandle<Wry>,
        preference: Arc<MobileNativeBackendState>,
        recovery: Arc<MobileBackendRecoveryState>,
    ) -> Self {
        Self {
            app,
            preference,
            recovery,
        }
    }

    pub(super) fn request(&self) -> bool {
        let recovery_id = match self.recovery.begin() {
            Ok(Some(recovery_id)) => recovery_id,
            Ok(None) => return false,
            Err(error) => {
                eprintln!("deve_mobile native recovery state failed closed: {error}");
                return false;
            }
        };
        let coordinator = self.clone();
        tauri::async_runtime::spawn(async move {
            match coordinator.switch_to_local(recovery_id).await {
                Ok(()) => {
                    if let Err(error) = coordinator.recovery.finish_success(recovery_id) {
                        eprintln!("deve_mobile native recovery completion failed closed: {error}");
                        coordinator.recovery.force_inactive();
                        coordinator.app.request_restart();
                    }
                }
                Err(mut failure) => {
                    eprintln!(
                        "deve_mobile native LocalBackend recovery failed closed: {}",
                        failure.error
                    );
                    if !failure.restart_required {
                        let reset = match coordinator
                            .app
                            .get_webview_window(MOBILE_TAURI_MAIN_WINDOW_LABEL)
                        {
                            Some(window) => reset_platform_recovery_control(&window).await,
                            None => {
                                Err("remote WebView unavailable during control reset".to_string())
                            }
                        };
                        if let Err(error) = reset {
                            eprintln!(
                                "deve_mobile native recovery control reset failed closed: {error}"
                            );
                            failure.restart_required = true;
                        }
                    }
                    let error = failure.error.to_string();
                    if let Err(state_error) = coordinator.recovery.finish_failure(
                        recovery_id,
                        error,
                        failure.active_runtime_owners,
                    ) {
                        eprintln!(
                            "deve_mobile native recovery failure state failed closed: {state_error}"
                        );
                        coordinator.recovery.force_inactive();
                        failure.restart_required = true;
                    }
                    if failure.restart_required {
                        coordinator.app.request_restart();
                    }
                }
            }
        });
        true
    }

    async fn switch_to_local(&self, recovery_id: u64) -> Result<(), RecoveryFailure> {
        let preference = self.preference.preference().map_err(|error| {
            RecoveryFailure::remote_active(MobileBackendRecoveryError::PreferenceWrite(
                error.to_string(),
            ))
        })?;
        if preference.mode != NativeBackendMode::Remote {
            return Err(RecoveryFailure::remote_active(
                MobileBackendRecoveryError::PreferenceNotRemote,
            ));
        }
        let remote_window = self
            .app
            .get_webview_window(MOBILE_TAURI_MAIN_WINDOW_LABEL)
            .ok_or_else(|| {
                RecoveryFailure::remote_active(MobileBackendRecoveryError::RemoteWindowUnavailable)
            })?;
        if self
            .app
            .try_state::<Arc<MobileEmbeddedBackendSupervisor>>()
            .is_some()
        {
            return Err(RecoveryFailure::remote_active(
                MobileBackendRecoveryError::SupervisorAlreadyManaged,
            ));
        }

        let app_data_dir = self.app.path().app_data_dir().map_err(|error| {
            RecoveryFailure::remote_active(MobileBackendRecoveryError::AppDataDirUnavailable(
                error.to_string(),
            ))
        })?;
        let (candidate, bootstrap) = tauri::async_runtime::spawn_blocking(move || {
            MobileEmbeddedBackendSupervisor::start(app_data_dir)
        })
        .await
        .map_err(|error| {
            RecoveryFailure::remote_active(MobileBackendRecoveryError::SupervisorStartTask(
                error.to_string(),
            ))
        })?
        .map_err(|error| {
            RecoveryFailure::remote_active(MobileBackendRecoveryError::SupervisorStart(
                error.to_string(),
            ))
        })?;
        if let Err(error) = self.record(recovery_id, MobileBackendRecoveryPhase::CandidateStarted) {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure {
                error,
                restart_required: active_runtime_owners != 0,
                active_runtime_owners,
            });
        }

        if let Err(error) = remove_platform_recovery_control(&remote_window).await {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure {
                error: MobileBackendRecoveryError::NativeControlRetire(error),
                restart_required: active_runtime_owners != 0,
                active_runtime_owners,
            });
        }
        if let Err(error) = self.record(
            recovery_id,
            MobileBackendRecoveryPhase::NativeControlRetired,
        ) {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure {
                error,
                restart_required: active_runtime_owners != 0,
                active_runtime_owners,
            });
        }

        if let Err(error) = remote_window.destroy() {
            let reset_failed = reset_platform_recovery_control(&remote_window)
                .await
                .is_err();
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure {
                error: MobileBackendRecoveryError::RemoteWindowDestroy(error.to_string()),
                restart_required: reset_failed || active_runtime_owners != 0,
                active_runtime_owners,
            });
        }
        if !wait_for_remote_window_retirement(&self.app).await {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure::after_retirement(
                MobileBackendRecoveryError::RemoteWindowRetireTimeout,
                active_runtime_owners,
            ));
        }
        if let Err(error) = self.record(
            recovery_id,
            MobileBackendRecoveryPhase::RemoteSurfaceRetired,
        ) {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure::after_retirement(
                error,
                active_runtime_owners,
            ));
        }

        if let Err(error) = self
            .preference
            .save_preference(NativeBackendPreference::local())
        {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure::after_retirement(
                MobileBackendRecoveryError::PreferenceWrite(error.to_string()),
                active_runtime_owners,
            ));
        }
        if let Err(error) =
            self.record(recovery_id, MobileBackendRecoveryPhase::PreferenceCommitted)
        {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure::after_retirement(
                error,
                active_runtime_owners,
            ));
        }

        if let Err(error) = self.app.plugin(mobile_local_backend_command_plugin()) {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure::after_retirement(
                MobileBackendRecoveryError::CommandPlugin(error.to_string()),
                active_runtime_owners,
            ));
        }
        if let Err(error) = self
            .app
            .plugin(mobile_embedded_backend_plugin(&bootstrap.script))
        {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure::after_retirement(
                MobileBackendRecoveryError::BootstrapPlugin(error.to_string()),
                active_runtime_owners,
            ));
        }
        if let Err(error) = self.record(
            recovery_id,
            MobileBackendRecoveryPhase::LocalPluginsRegistered,
        ) {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure::after_retirement(
                error,
                active_runtime_owners,
            ));
        }

        let supervisor = Arc::new(candidate);
        if !self.app.manage(supervisor.clone()) {
            let active_runtime_owners = u8::from(!shutdown_candidate(&supervisor).await);
            return Err(RecoveryFailure::after_retirement(
                MobileBackendRecoveryError::SupervisorAlreadyManaged,
                active_runtime_owners,
            ));
        }
        if let Err(error) = self.record(recovery_id, MobileBackendRecoveryPhase::SupervisorManaged)
        {
            let active_runtime_owners = u8::from(!shutdown_candidate(&supervisor).await);
            return Err(RecoveryFailure::after_retirement(
                error,
                active_runtime_owners,
            ));
        }

        if let Err(error) = create_mobile_main_window(&self.app, None) {
            let active_runtime_owners = u8::from(!shutdown_candidate(&supervisor).await);
            return Err(RecoveryFailure::after_retirement(
                MobileBackendRecoveryError::LocalWindowCreate(error),
                active_runtime_owners,
            ));
        }
        if let Err(error) = self.record(recovery_id, MobileBackendRecoveryPhase::LocalWindowCreated)
        {
            if let Some(window) = self.app.get_webview_window(MOBILE_TAURI_MAIN_WINDOW_LABEL) {
                let _ = window.destroy();
            }
            let active_runtime_owners = u8::from(!shutdown_candidate(&supervisor).await);
            return Err(RecoveryFailure::after_retirement(
                error,
                active_runtime_owners,
            ));
        }
        eprintln!(
            "deve_mobile RemoteBrowser recovered to fresh LocalBackend runtime recovery_id={recovery_id}"
        );
        Ok(())
    }

    fn record(
        &self,
        recovery_id: u64,
        phase: MobileBackendRecoveryPhase,
    ) -> Result<(), MobileBackendRecoveryError> {
        self.recovery
            .record_phase(recovery_id, phase)
            .map_err(MobileBackendRecoveryError::TransitionState)
    }
}

async fn shutdown_candidate(supervisor: &MobileEmbeddedBackendSupervisor) -> bool {
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

async fn wait_for_remote_window_retirement(app: &AppHandle<Wry>) -> bool {
    let deadline = tokio::time::Instant::now() + REMOTE_WINDOW_RETIRE_TIMEOUT;
    loop {
        if app
            .get_webview_window(MOBILE_TAURI_MAIN_WINDOW_LABEL)
            .is_none()
        {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
