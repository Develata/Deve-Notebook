//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use serde::{Deserialize, Serialize};

pub const NATIVE_SESSION_BOOTSTRAP_SECRET_ENV: &str = "DEVE_NATIVE_SESSION_BOOTSTRAP_SECRET";
pub const NATIVE_SESSION_BOOTSTRAP_HEADER: &str = "x-deve-native-session-secret";
pub const NATIVE_TAURI_HTTP_LOCALHOST_ORIGIN: &str = "http://tauri.localhost";
pub const NATIVE_TAURI_CUSTOM_PROTOCOL_ORIGIN: &str = "tauri://localhost";

pub fn native_tauri_allowed_origins() -> Vec<String> {
    vec![
        NATIVE_TAURI_HTTP_LOCALHOST_ORIGIN.to_string(),
        NATIVE_TAURI_CUSTOM_PROTOCOL_ORIGIN.to_string(),
    ]
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum NativeShellMode {
    LocalBackend,
    RemoteBrowser { target: NativeRemoteTarget },
}

impl NativeShellMode {
    pub fn local_backend_default() -> Self {
        Self::LocalBackend
    }

    pub fn remote_browser(https_origin: impl Into<String>) -> Self {
        Self::RemoteBrowser {
            target: NativeRemoteTarget {
                https_origin: https_origin.into(),
            },
        }
    }

    pub fn starts_local_backend(&self) -> bool {
        matches!(self, Self::LocalBackend)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRemoteTarget {
    pub https_origin: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBackendMode {
    Local,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBackendPreference {
    pub mode: NativeBackendMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
}

impl Default for NativeBackendPreference {
    fn default() -> Self {
        Self::local()
    }
}

impl NativeBackendPreference {
    pub fn local() -> Self {
        Self {
            mode: NativeBackendMode::Local,
            remote_url: None,
        }
    }

    pub fn remote(https_origin: impl Into<String>) -> Self {
        Self {
            mode: NativeBackendMode::Remote,
            remote_url: Some(https_origin.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBackendValidationResult {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub https_origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl NativeBackendValidationResult {
    pub fn success(https_origin: impl Into<String>, node_role: impl Into<String>) -> Self {
        Self {
            ok: true,
            https_origin: Some(https_origin.into()),
            node_role: Some(node_role.into()),
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            https_origin: None,
            node_role: None,
            error: Some(error.into()),
        }
    }
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
