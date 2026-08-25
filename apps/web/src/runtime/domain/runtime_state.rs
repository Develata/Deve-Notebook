//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! Runtime display and wire-state contract shared by Web runtime clients.

use deve_core::models::{PeerId, VersionVector};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

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
    /// Stable identity owned by the Web UI; it is never sent over the wire.
    pub ui_id: u64,
    pub role: String,
    pub content: String,
    pub req_id: Option<String>,
    pub ts_ms: u64,
    /// Monotonic local revision used to invalidate only this row's content projection.
    pub content_revision: u64,
}

impl ChatMessage {
    pub fn new(
        role: impl Into<String>,
        content: impl Into<String>,
        req_id: Option<String>,
        ts_ms: u64,
    ) -> Self {
        static NEXT_UI_ID: AtomicU64 = AtomicU64::new(1);
        let ui_id = NEXT_UI_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .expect("chat UI message identity exhausted");
        Self {
            ui_id,
            role: role.into(),
            content: content.into(),
            req_id,
            ts_ms,
            content_revision: 0,
        }
    }

    pub fn append_content(&mut self, delta: &str) -> bool {
        if delta.is_empty() {
            return false;
        }
        self.content.push_str(delta);
        self.content_revision = self
            .content_revision
            .checked_add(1)
            .expect("chat message content revision exhausted");
        true
    }
}

#[cfg(test)]
mod chat_message_tests {
    use super::ChatMessage;

    #[test]
    fn chat_message_ui_identity_is_unique_for_same_timestamp_and_content() {
        let first = ChatMessage::new("assistant", "same prefix", None, 42);
        let second = ChatMessage::new("assistant", "same prefix", None, 42);

        assert_ne!(first.ui_id, second.ui_id);
    }

    #[test]
    fn chat_message_append_preserves_identity_and_advances_content_revision() {
        let mut message = ChatMessage::new("assistant", "prefix", Some("req-1".into()), 42);
        let ui_id = message.ui_id;

        assert!(message.append_content(" delta"));
        assert_eq!(message.ui_id, ui_id);
        assert_eq!(message.content, "prefix delta");
        assert_eq!(message.content_revision, 1);
        assert!(!message.append_content(""));
        assert_eq!(message.content_revision, 1);
    }
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
    Resyncing,
    Partial,
    Error,
}

impl LoadPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Loading => "loading",
            Self::Resyncing => "resyncing",
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
            "resyncing" => Some(Self::Resyncing),
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
