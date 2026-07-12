//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! Runtime display and wire-state contract shared by Web runtime clients.

use deve_core::models::{PeerId, VersionVector};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AiBackendMode {
    #[default]
    Native,
    TrustedCli,
}

impl AiBackendMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => crate::api::AI_BACKEND_NATIVE,
            Self::TrustedCli => crate::api::AI_BACKEND_TRUSTED_CLI,
        }
    }

    pub fn from_backend_str(value: &str) -> Option<Self> {
        match value {
            crate::api::AI_BACKEND_NATIVE => Some(Self::Native),
            crate::api::AI_BACKEND_TRUSTED_CLI => Some(Self::TrustedCli),
            _ => None,
        }
    }

    pub fn from_backend_str_or_native(value: &str) -> Self {
        Self::from_backend_str(value).unwrap_or(Self::Native)
    }

    pub fn plugin_id(self) -> &'static str {
        match self {
            Self::Native => crate::api::AI_PLUGIN_NATIVE,
            Self::TrustedCli => crate::api::AI_PLUGIN_TRUSTED_CLI,
        }
    }
}

impl fmt::Display for AiBackendMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<&str> for AiBackendMode {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub req_id: Option<String>,
    pub ts_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeerSession {
    pub id: PeerId,
    pub vector: VersionVector,
    pub last_seen: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoadPhase {
    #[default]
    Ready,
    Loading,
    Partial,
    Error,
}

impl LoadPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Loading => "loading",
            Self::Partial => "partial",
            Self::Error => "error",
        }
    }

    pub fn is_ready(self) -> bool {
        self == Self::Ready
    }
}

#[cfg(test)]
impl LoadPhase {
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "ready" => Some(Self::Ready),
            "loading" => Some(Self::Loading),
            "partial" => Some(Self::Partial),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    pub fn from_wire_or_ready(value: &str) -> Self {
        Self::from_wire(value).unwrap_or(Self::Ready)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorSyncFailureCode {
    SnapshotApply,
    DeltaReplay,
    HistoryReplay,
    LiveReplay,
    ContentReadback,
}

impl EditorSyncFailureCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SnapshotApply => "snapshot_apply",
            Self::DeltaReplay => "delta_replay",
            Self::HistoryReplay => "history_replay",
            Self::LiveReplay => "live_replay",
            Self::ContentReadback => "content_readback",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorSyncFailure {
    pub code: EditorSyncFailureCode,
    pub generation: u64,
    pub request_id: u64,
}

impl EditorSyncFailure {
    pub const fn new(code: EditorSyncFailureCode, generation: u64, request_id: u64) -> Self {
        Self {
            code,
            generation,
            request_id,
        }
    }
}

impl fmt::Display for LoadPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<&str> for LoadPhase {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SyncModeState {
    #[default]
    Auto,
    Manual,
}

impl SyncModeState {
    pub fn as_str(self) -> &'static str {
        self.as_wire()
    }

    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }

    pub fn from_wire_or_auto(value: &str) -> Self {
        Self::from_wire(value).unwrap_or(Self::Auto)
    }
}

impl fmt::Display for SyncModeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_wire())
    }
}

impl PartialEq<&str> for SyncModeState {
    fn eq(&self, other: &&str) -> bool {
        self.as_wire() == *other
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub doc_id: String,
    pub path: String,
    pub score: f32,
}

impl SearchHit {
    pub fn new(doc_id: String, path: String, score: f32) -> Self {
        Self {
            doc_id,
            path,
            score,
        }
    }
}

impl From<(String, String, f32)> for SearchHit {
    fn from((doc_id, path, score): (String, String, f32)) -> Self {
        Self::new(doc_id, path, score)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingOpsPreview {
    pub path: String,
    pub old_preview: String,
    pub new_preview: String,
}

impl PendingOpsPreview {
    pub fn new(path: String, old_preview: String, new_preview: String) -> Self {
        Self {
            path,
            old_preview,
            new_preview,
        }
    }
}

impl From<(String, String, String)> for PendingOpsPreview {
    fn from((path, old_preview, new_preview): (String, String, String)) -> Self {
        Self::new(path, old_preview, new_preview)
    }
}
