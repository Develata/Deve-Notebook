//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!
//! Serialized supervisor state and current-generation WebView handoff payload.

#![cfg_attr(not(mobile), allow(dead_code))]

use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::oneshot;

#[cfg(mobile)]
use super::MobileEmbeddedBackendBootstrap;
use super::cookie::MobileNativeSessionCookie;
#[cfg(mobile)]
use super::cookie::install_native_session_cookie_confirmed;
use super::generation::{BackendTask, backend_requires_restart};
use super::{MobileEmbeddedBackendError, MobileEmbeddedBackendPlan};
use crate::MobileShell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileEmbeddedBackendServiceState {
    EndpointSessionReady,
    BackgroundSuspended,
    ForegroundReprobe,
    Stopping,
    Stopped,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MobileEmbeddedBackendSupervisorSnapshot {
    pub endpoint: Option<String>,
    pub session_generation: u64,
    pub service_state: MobileEmbeddedBackendServiceState,
    pub backend_running: bool,
    pub last_error: Option<String>,
}

pub struct MobileEmbeddedBackendResume {
    #[cfg(mobile)]
    pub(super) native_session_cookie: MobileNativeSessionCookie,
    #[cfg(mobile)]
    pub(super) replacement_bootstrap: Option<MobileEmbeddedBackendBootstrap>,
    pub restarted: bool,
    pub session_generation: u64,
    #[cfg_attr(not(mobile), allow(dead_code))]
    pub(super) transition_token: u64,
}

pub(super) struct BackendGeneration {
    pub(super) runtime: Option<deve_cli::native_runtime::NativeEmbeddedServerRuntime>,
    pub(super) plan: MobileEmbeddedBackendPlan,
    pub(super) native_session_cookie: MobileNativeSessionCookie,
    pub(super) task: Option<BackendTask>,
    pub(super) shutdown_sender: Option<oneshot::Sender<()>>,
    pub(super) transport_stopping: bool,
    pub(super) runtime_restart_required: bool,
    pub(super) probe_cancel: Option<Arc<AtomicBool>>,
    pub(super) shell: MobileShell,
    pub(super) session_generation: u64,
    pub(super) transition_token: u64,
    pub(super) service_state: MobileEmbeddedBackendServiceState,
    pub(super) last_error: Option<String>,
    pub(super) last_error_transition_token: Option<u64>,
}

impl MobileEmbeddedBackendResume {
    #[cfg(mobile)]
    pub(super) async fn install_on_webview<R: tauri::Runtime>(
        &self,
        webview: &tauri::WebviewWindow<R>,
    ) -> Result<(), String> {
        install_native_session_cookie_confirmed(webview, &self.native_session_cookie).await?;
        if let Some(bootstrap) = self.replacement_bootstrap.as_ref() {
            // No reload: the current endpoint is projected into the existing
            // WebView and persisted only for this WebView session.
            webview
                .eval(bootstrap.script.replacement_source())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

pub(super) fn next_transition_token(current: u64) -> Result<u64, MobileEmbeddedBackendError> {
    current
        .checked_add(1)
        .ok_or(MobileEmbeddedBackendError::SessionGenerationOverflow)
}

pub(super) fn ensure_current_transition(
    generation: &BackendGeneration,
    transition_token: u64,
) -> Result<(), MobileEmbeddedBackendError> {
    if generation.transition_token != transition_token
        || matches!(
            generation.service_state,
            MobileEmbeddedBackendServiceState::Stopping
                | MobileEmbeddedBackendServiceState::Stopped
        )
    {
        return Err(MobileEmbeddedBackendError::LifecycleTransitionCancelled);
    }
    Ok(())
}

#[cfg_attr(not(any(mobile, test)), allow(dead_code))]
pub(super) fn ensure_current_resume(
    generation: &BackendGeneration,
    resumed: &MobileEmbeddedBackendResume,
) -> Result<(), MobileEmbeddedBackendError> {
    if generation.transition_token != resumed.transition_token
        || generation.session_generation != resumed.session_generation
        || generation.service_state != MobileEmbeddedBackendServiceState::EndpointSessionReady
    {
        return Err(MobileEmbeddedBackendError::LifecycleTransitionCancelled);
    }
    Ok(())
}

pub(super) fn snapshot_from_generation(
    generation: &BackendGeneration,
) -> MobileEmbeddedBackendSupervisorSnapshot {
    MobileEmbeddedBackendSupervisorSnapshot {
        endpoint: Some(generation.plan.http_base.clone()),
        session_generation: generation.session_generation,
        service_state: generation.service_state,
        backend_running: generation.runtime.is_some()
            && generation.shutdown_sender.is_some()
            && !generation.transport_stopping
            && !backend_requires_restart(generation.task.as_ref()),
        last_error: generation.last_error.clone(),
    }
}

pub(super) fn record_error(generation: &mut BackendGeneration, error: &MobileEmbeddedBackendError) {
    generation.service_state = MobileEmbeddedBackendServiceState::Error;
    generation.last_error = Some(error.to_string());
    generation
        .shell
        .mark_service_offline(error.to_string(), false);
}
