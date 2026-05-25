//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use serde::{Deserialize, Serialize};

pub const NATIVE_SESSION_BOOTSTRAP_SECRET_ENV: &str = "DEVE_NATIVE_SESSION_BOOTSTRAP_SECRET";
pub const NATIVE_SESSION_BOOTSTRAP_HEADER: &str = "x-deve-native-session-secret";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeAdapterPlatform {
    Desktop,
    Mobile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeAdapterState {
    ColdStart,
    ServiceStarting,
    EndpointBound,
    SessionBound,
    WebShellLoading,
    RuntimeReady,
    ServiceRestarting,
    ServiceOffline,
    SessionInvalid,
    BackgroundSuspended,
    ForegroundReprobe,
}

impl NativeAdapterState {
    pub fn is_writable_candidate(self) -> bool {
        self == Self::RuntimeReady
    }

    pub fn requires_unauthorized_ui(self) -> bool {
        self == Self::SessionInvalid
    }

    pub fn requires_recovery_ui(self) -> bool {
        matches!(self, Self::ServiceOffline | Self::ServiceRestarting)
    }

    pub fn requires_fresh_handshake(self) -> bool {
        matches!(
            self,
            Self::ColdStart
                | Self::ServiceStarting
                | Self::EndpointBound
                | Self::SessionBound
                | Self::WebShellLoading
                | Self::ServiceRestarting
                | Self::ServiceOffline
                | Self::SessionInvalid
                | Self::BackgroundSuspended
                | Self::ForegroundReprobe
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeEndpointReady {
    pub http_base: String,
    pub ws_base: String,
    pub node_role: String,
    pub session_bound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeServiceOffline {
    pub reason: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeServiceRestarting {
    pub attempt: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeServiceSuspended {
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePlatformEventKind {
    WindowFocused,
    WindowBlurred,
    ThemeChanged,
    NetworkOnline,
    NetworkOffline,
    CloseRequested,
    BackgroundRequested,
    Foreground,
    Background,
    Suspended,
    Resumed,
    SafeAreaChanged,
    KeyboardChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePlatformEventEffect {
    NoBusinessStateChange,
    NetworkHintOnly,
    RequestCloseOrBackground,
    EnterBackgroundSuspended,
    RequireForegroundReprobe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativePlatformEvent {
    pub kind: NativePlatformEventKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRuntimeReadiness {
    pub endpoint_reachable: bool,
    pub auth_status_valid: bool,
    pub node_role_readable: bool,
    pub repo_handshake_complete: bool,
    pub writer_ready: bool,
    pub scope_nonce_current: bool,
}

impl NativeRuntimeReadiness {
    pub fn is_runtime_ready(self) -> bool {
        self.endpoint_reachable
            && self.auth_status_valid
            && self.node_role_readable
            && self.repo_handshake_complete
            && self.writer_ready
            && self.scope_nonce_current
    }

    pub fn needs_reprobe_before_write(self) -> bool {
        !self.is_runtime_ready()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeAdapterSnapshot {
    pub platform: NativeAdapterPlatform,
    pub state: NativeAdapterState,
    pub endpoint: Option<NativeEndpointReady>,
    pub readiness: NativeRuntimeReadiness,
}

impl NativeAdapterSnapshot {
    pub fn unauthorized_or_recovery_gate(&self) -> bool {
        self.state.requires_unauthorized_ui() || self.state.requires_recovery_ui()
    }
}

pub fn classify_native_platform_event(
    platform: NativeAdapterPlatform,
    event: NativePlatformEventKind,
) -> NativePlatformEventEffect {
    match event {
        NativePlatformEventKind::NetworkOnline | NativePlatformEventKind::NetworkOffline => {
            NativePlatformEventEffect::NetworkHintOnly
        }
        NativePlatformEventKind::CloseRequested | NativePlatformEventKind::BackgroundRequested => {
            NativePlatformEventEffect::RequestCloseOrBackground
        }
        NativePlatformEventKind::Foreground | NativePlatformEventKind::Resumed => {
            NativePlatformEventEffect::RequireForegroundReprobe
        }
        NativePlatformEventKind::Background | NativePlatformEventKind::Suspended
            if platform == NativeAdapterPlatform::Mobile =>
        {
            NativePlatformEventEffect::EnterBackgroundSuspended
        }
        _ => NativePlatformEventEffect::NoBusinessStateChange,
    }
}

pub fn platform_event_can_grant_write(_event: NativePlatformEventKind) -> bool {
    false
}
