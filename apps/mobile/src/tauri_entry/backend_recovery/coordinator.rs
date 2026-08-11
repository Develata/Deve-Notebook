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
use crate::embedded_backend::{MobileEmbeddedBackendSupervisor, mobile_embedded_backend_plugin};

use super::super::{MOBILE_TAURI_MAIN_WINDOW_LABEL, mobile_local_backend_command_plugin};
use super::cleanup::{
    retire_and_confirm_recovery_anchor, shutdown_candidate, shutdown_managed_supervisor,
    wait_for_window_retirement,
};
use super::state::{MobileBackendRecoveryPhase, MobileBackendRecoveryState};
use super::{
    PlatformColdRestartSource, PlatformRecoveryAnchor, create_platform_local_main_window,
    create_platform_recovery_anchor, remove_platform_recovery_control,
    request_platform_cold_restart, reset_platform_recovery_control, retire_platform_remote_surface,
};

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
    #[error("mobile Android recovery lifecycle anchor creation failed: {0}")]
    RecoveryAnchorCreate(String),
    #[error("mobile RemoteBrowser surface retirement failed: {0}")]
    RemoteSurfaceRetire(String),
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
    #[error("mobile LocalBackend WebView handoff admission failed closed")]
    WebviewHandoffAdmission,
    #[error("mobile Android recovery lifecycle anchor did not retire")]
    RecoveryAnchorRetire,
    #[error("mobile LocalBackend recovery transition state failed: {0}")]
    TransitionState(String),
}

struct RecoveryFailure {
    error: MobileBackendRecoveryError,
    restart_required: bool,
    restart_source: PlatformColdRestartSource,
    active_runtime_owners: u8,
}

impl RecoveryFailure {
    fn remote_active(error: MobileBackendRecoveryError) -> Self {
        Self {
            error,
            restart_required: false,
            restart_source: PlatformColdRestartSource::Main,
            active_runtime_owners: 0,
        }
    }

    fn after_retirement(error: MobileBackendRecoveryError, active_runtime_owners: u8) -> Self {
        Self {
            error,
            restart_required: true,
            restart_source: PlatformColdRestartSource::RecoveryAnchor,
            active_runtime_owners,
        }
    }

    fn after_retirement_from_main(
        error: MobileBackendRecoveryError,
        active_runtime_owners: u8,
    ) -> Self {
        Self {
            error,
            restart_required: true,
            restart_source: PlatformColdRestartSource::Main,
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
                        if !shutdown_managed_supervisor(&coordinator.app).await {
                            eprintln!(
                                "deve_mobile managed LocalBackend shutdown remained unconfirmed before forced retirement"
                            );
                        }
                        coordinator.recovery.force_inactive();
                        request_platform_cold_restart(
                            &coordinator.app,
                            PlatformColdRestartSource::Main,
                        )
                        .await;
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
                        request_platform_cold_restart(&coordinator.app, failure.restart_source)
                            .await;
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
        if candidate
            .defer_initial_webview_session_for_recovery()
            .is_err()
        {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure {
                error: MobileBackendRecoveryError::WebviewHandoffAdmission,
                restart_required: active_runtime_owners != 0,
                restart_source: PlatformColdRestartSource::Main,
                active_runtime_owners,
            });
        }
        if let Err(error) = self.record(recovery_id, MobileBackendRecoveryPhase::CandidateStarted) {
            let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
            return Err(RecoveryFailure {
                error,
                restart_required: active_runtime_owners != 0,
                restart_source: PlatformColdRestartSource::Main,
                active_runtime_owners,
            });
        }

        let recovery_anchor = match create_platform_recovery_anchor(&self.app, &remote_window) {
            Ok(anchor) => anchor,
            Err(error) => {
                let active_runtime_owners = u8::from(!shutdown_candidate(&candidate).await);
                return Err(RecoveryFailure {
                    error: MobileBackendRecoveryError::RecoveryAnchorCreate(error),
                    // Android Activity creation can commit after the builder's
                    // bounded acknowledgement wait. Never reopen the remote
                    // control from this committed-unknown state.
                    restart_required: true,
                    restart_source: PlatformColdRestartSource::Main,
                    active_runtime_owners,
                });
            }
        };

        if let Err(error) = remove_platform_recovery_control(&remote_window).await {
            return Err(failure_before_remote_retirement(
                &self.app,
                MobileBackendRecoveryError::NativeControlRetire(error),
                &recovery_anchor,
                &candidate,
            )
            .await);
        }
        if let Err(error) = self.record(
            recovery_id,
            MobileBackendRecoveryPhase::NativeControlRetired,
        ) {
            return Err(failure_before_remote_retirement(
                &self.app,
                error,
                &recovery_anchor,
                &candidate,
            )
            .await);
        }

        let retire_dispatch_error = retire_platform_remote_surface(&remote_window).await.err();
        if !wait_for_window_retirement(&self.app, MOBILE_TAURI_MAIN_WINDOW_LABEL).await {
            let error = retire_dispatch_error.map_or(
                MobileBackendRecoveryError::RemoteWindowRetireTimeout,
                MobileBackendRecoveryError::RemoteSurfaceRetire,
            );
            return Err(failure_after_remote_retirement(
                &self.app,
                error,
                &recovery_anchor,
                &candidate,
            )
            .await);
        }
        if retire_dispatch_error.is_some() {
            eprintln!(
                "deve_mobile remote surface retirement committed despite an unconfirmed dispatch"
            );
        }
        if let Err(error) = self.record(
            recovery_id,
            MobileBackendRecoveryPhase::RemoteSurfaceRetired,
        ) {
            return Err(failure_after_remote_retirement(
                &self.app,
                error,
                &recovery_anchor,
                &candidate,
            )
            .await);
        }

        if let Err(error) = self
            .preference
            .save_preference(NativeBackendPreference::local())
        {
            return Err(failure_after_remote_retirement(
                &self.app,
                MobileBackendRecoveryError::PreferenceWrite(error.to_string()),
                &recovery_anchor,
                &candidate,
            )
            .await);
        }
        if let Err(error) =
            self.record(recovery_id, MobileBackendRecoveryPhase::PreferenceCommitted)
        {
            return Err(failure_after_remote_retirement(
                &self.app,
                error,
                &recovery_anchor,
                &candidate,
            )
            .await);
        }

        if let Err(error) = self.app.plugin(mobile_local_backend_command_plugin()) {
            return Err(failure_after_remote_retirement(
                &self.app,
                MobileBackendRecoveryError::CommandPlugin(error.to_string()),
                &recovery_anchor,
                &candidate,
            )
            .await);
        }
        if let Err(error) = self
            .app
            .plugin(mobile_embedded_backend_plugin(&bootstrap.script))
        {
            return Err(failure_after_remote_retirement(
                &self.app,
                MobileBackendRecoveryError::BootstrapPlugin(error.to_string()),
                &recovery_anchor,
                &candidate,
            )
            .await);
        }
        if let Err(error) = self.record(
            recovery_id,
            MobileBackendRecoveryPhase::LocalPluginsRegistered,
        ) {
            return Err(failure_after_remote_retirement(
                &self.app,
                error,
                &recovery_anchor,
                &candidate,
            )
            .await);
        }

        let supervisor = Arc::new(candidate);
        if !self.app.manage(supervisor.clone()) {
            return Err(failure_after_remote_retirement(
                &self.app,
                MobileBackendRecoveryError::SupervisorAlreadyManaged,
                &recovery_anchor,
                &supervisor,
            )
            .await);
        }
        if let Err(error) = self.record(recovery_id, MobileBackendRecoveryPhase::SupervisorManaged)
        {
            return Err(failure_after_remote_retirement(
                &self.app,
                error,
                &recovery_anchor,
                &supervisor,
            )
            .await);
        }

        if let Err(error) = create_platform_local_main_window(&self.app, &recovery_anchor) {
            return Err(failure_after_remote_retirement(
                &self.app,
                MobileBackendRecoveryError::LocalWindowCreate(error),
                &recovery_anchor,
                &supervisor,
            )
            .await);
        }
        if !retire_and_confirm_recovery_anchor(&self.app, &recovery_anchor).await {
            let active_runtime_owners = u8::from(!shutdown_candidate(&supervisor).await);
            return Err(RecoveryFailure::after_retirement_from_main(
                MobileBackendRecoveryError::RecoveryAnchorRetire,
                active_runtime_owners,
            ));
        }
        if let Err(error) = self.record(recovery_id, MobileBackendRecoveryPhase::LocalWindowCreated)
        {
            let active_runtime_owners = u8::from(!shutdown_candidate(&supervisor).await);
            return Err(RecoveryFailure::after_retirement_from_main(
                error,
                active_runtime_owners,
            ));
        }
        if supervisor
            .admit_initial_webview_session_after_recovery()
            .is_err()
        {
            let active_runtime_owners = u8::from(!shutdown_candidate(&supervisor).await);
            return Err(RecoveryFailure::after_retirement_from_main(
                MobileBackendRecoveryError::WebviewHandoffAdmission,
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

async fn failure_before_remote_retirement(
    app: &AppHandle<Wry>,
    error: MobileBackendRecoveryError,
    recovery_anchor: &PlatformRecoveryAnchor,
    supervisor: &MobileEmbeddedBackendSupervisor,
) -> RecoveryFailure {
    let anchor_retired = retire_and_confirm_recovery_anchor(app, recovery_anchor).await;
    let active_runtime_owners = u8::from(!shutdown_candidate(supervisor).await);
    RecoveryFailure {
        error,
        restart_required: !anchor_retired || active_runtime_owners != 0,
        restart_source: PlatformColdRestartSource::Main,
        active_runtime_owners,
    }
}

async fn failure_after_remote_retirement(
    _app: &AppHandle<Wry>,
    error: MobileBackendRecoveryError,
    _recovery_anchor: &PlatformRecoveryAnchor,
    supervisor: &MobileEmbeddedBackendSupervisor,
) -> RecoveryFailure {
    let active_runtime_owners = u8::from(!shutdown_candidate(supervisor).await);
    RecoveryFailure::after_retirement(error, active_runtime_owners)
}
