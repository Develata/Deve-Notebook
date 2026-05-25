//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::{
    NativeEndpointReady, NativePlatformEventKind, NativeProcessAdapterError,
    NativeProcessAdapterSnapshot, NativeRuntimeReadiness, NativeServiceOffline,
    NativeServiceRestarting, NativeServiceSupervisorError, NativeServiceSupervisorSnapshot,
    NativeServiceSuspended,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MobileServiceState {
    ColdStart,
    ServiceStarting,
    EndpointBound,
    SessionBound,
    WebShellLoading,
    RuntimeReady,
    ServiceRestarting,
    BackgroundSuspended,
    ForegroundReprobe,
    ServiceOffline,
    SessionInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileLifecycleEvent {
    NetworkOnline,
    NetworkOffline,
    Background,
    Suspended,
    Resumed,
    Foreground,
    SafeAreaChanged,
    KeyboardChanged,
}

impl MobileLifecycleEvent {
    pub(super) fn to_native_kind(self) -> NativePlatformEventKind {
        match self {
            Self::NetworkOnline => NativePlatformEventKind::NetworkOnline,
            Self::NetworkOffline => NativePlatformEventKind::NetworkOffline,
            Self::Background => NativePlatformEventKind::Background,
            Self::Suspended => NativePlatformEventKind::Suspended,
            Self::Resumed => NativePlatformEventKind::Resumed,
            Self::Foreground => NativePlatformEventKind::Foreground,
            Self::SafeAreaChanged => NativePlatformEventKind::SafeAreaChanged,
            Self::KeyboardChanged => NativePlatformEventKind::KeyboardChanged,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileShellSnapshot {
    pub state: MobileServiceState,
    pub endpoint: Option<NativeEndpointReady>,
    pub readiness: NativeRuntimeReadiness,
    pub offline: Option<NativeServiceOffline>,
    pub restarting: Option<NativeServiceRestarting>,
    pub suspended: Option<NativeServiceSuspended>,
    pub supervisor: NativeServiceSupervisorSnapshot,
    pub process_adapter: NativeProcessAdapterSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileSessionMaterial {
    pub(super) bound: bool,
}

impl MobileSessionMaterial {
    pub fn bound() -> Self {
        Self { bound: true }
    }

    pub fn pending() -> Self {
        Self { bound: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MobileBootstrap {
    pub http_base: String,
    pub ws_base: String,
    pub node_role: String,
    pub session_bound: bool,
}

impl MobileBootstrap {
    pub fn script_tag(&self) -> Result<String, MobileShellError> {
        let payload = serde_json::to_string(self)?;
        Ok(format!(
            "<script>window.__DEVE_NATIVE_BOOTSTRAP={payload};</script>"
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MobileRecoveryBootstrap {
    pub service_state: &'static str,
}

impl MobileRecoveryBootstrap {
    pub fn script_tag(&self) -> Result<String, MobileShellError> {
        let payload = serde_json::to_string(self)?;
        Ok(format!(
            "<script>window.__DEVE_NATIVE_BOOTSTRAP={payload};</script>"
        ))
    }
}

#[derive(Debug, Error)]
pub enum MobileShellError {
    #[error("mobile service endpoint is invalid: {0}")]
    InvalidEndpoint(#[from] deve_core::native_adapter::NativeAdapterError),
    #[error("mobile session is not bound")]
    SessionNotBound,
    #[error("mobile foreground reprobe is required")]
    ForegroundReprobeRequired,
    #[error("mobile service is offline: {reason}")]
    ServiceOffline { reason: String },
    #[error("mobile session is invalid")]
    SessionInvalid,
    #[error("mobile service supervisor rejected transition: {0}")]
    Supervisor(#[from] NativeServiceSupervisorError),
    #[error("mobile process adapter rejected transition: {0}")]
    ProcessAdapter(#[from] NativeProcessAdapterError),
    #[error("failed to serialize mobile bootstrap: {0}")]
    BootstrapSerialize(#[from] serde_json::Error),
}
