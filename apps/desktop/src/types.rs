//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract

use deve_core::native_adapter::{
    NativeAdapterError, NativeEndpointReady, NativeProcessAdapterError,
    NativeProcessAdapterSnapshot, NativeRuntimeReadiness, NativeServiceOffline,
    NativeServiceRestarting, NativeServiceSupervisorError, NativeServiceSupervisorSnapshot,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopServiceState {
    ColdStart,
    ServiceStarting,
    EndpointBound,
    SessionBound,
    WebShellLoading,
    RuntimeReady,
    ServiceRestarting,
    ServiceOffline,
    SessionInvalid,
    ForegroundReprobe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopShellSnapshot {
    pub state: DesktopServiceState,
    pub endpoint: Option<NativeEndpointReady>,
    pub readiness: NativeRuntimeReadiness,
    pub offline: Option<NativeServiceOffline>,
    pub restarting: Option<NativeServiceRestarting>,
    pub supervisor: NativeServiceSupervisorSnapshot,
    pub process_adapter: NativeProcessAdapterSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopSessionMaterial {
    pub(super) bound: bool,
}

impl DesktopSessionMaterial {
    pub fn bound() -> Self {
        Self { bound: true }
    }

    pub fn pending() -> Self {
        Self { bound: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopBootstrap {
    pub http_base: String,
    pub ws_base: String,
    pub node_role: String,
    pub session_bound: bool,
}

impl DesktopBootstrap {
    pub fn script_source(&self) -> Result<String, DesktopShellError> {
        let payload = serde_json::to_string(self)?;
        Ok(format!("window.__DEVE_NATIVE_BOOTSTRAP={payload};"))
    }

    pub fn script_tag(&self) -> Result<String, DesktopShellError> {
        Ok(format!("<script>{}</script>", self.script_source()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopRecoveryBootstrap {
    pub service_state: &'static str,
}

impl DesktopRecoveryBootstrap {
    pub fn script_source(&self) -> Result<String, DesktopShellError> {
        let payload = serde_json::to_string(self)?;
        Ok(format!("window.__DEVE_NATIVE_BOOTSTRAP={payload};"))
    }

    pub fn script_tag(&self) -> Result<String, DesktopShellError> {
        Ok(format!("<script>{}</script>", self.script_source()?))
    }
}

#[derive(Debug, Error)]
pub enum DesktopShellError {
    #[error("desktop service endpoint is invalid: {0}")]
    InvalidEndpoint(#[from] NativeAdapterError),
    #[error("desktop session is not bound")]
    SessionNotBound,
    #[error("desktop service is offline: {reason}")]
    ServiceOffline { reason: String },
    #[error("desktop session is invalid")]
    SessionInvalid,
    #[error("desktop foreground reprobe is required before loading writable shell")]
    ForegroundReprobeRequired,
    #[error("desktop service supervisor rejected transition: {0}")]
    Supervisor(#[from] NativeServiceSupervisorError),
    #[error("desktop process adapter rejected transition: {0}")]
    ProcessAdapter(#[from] NativeProcessAdapterError),
    #[error("failed to serialize desktop bootstrap: {0}")]
    BootstrapSerialize(#[from] serde_json::Error),
}
