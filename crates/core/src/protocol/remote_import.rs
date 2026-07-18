//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!   - 06_backup#remote-import-session-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Sanitized Remote Import wire DTOs. These types intentionally cannot carry
//! locators, provider paths, artifact paths, digests, credentials or raw
//! backend failure detail.

use super::{RemoteProjectionProvider, ScopeNonce, ServerError};
use crate::models::{PeerId, RepoId};
use crate::source_control::diff_projection::DiffProjection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub const REMOTE_IMPORT_DEFAULT_PAGE_SIZE: u16 = 100;
pub const REMOTE_IMPORT_MAX_PAGE_SIZE: u16 = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteImportRequestContext {
    pub request_id: Uuid,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: ScopeNonce,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteImportResponseContext {
    pub request_id: Uuid,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: ScopeNonce,
    pub session_id: Option<RemoteImportSessionId>,
    pub revision: Option<RemoteImportCandidateRevision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RemoteImportSessionId(Uuid);

impl RemoteImportSessionId {
    pub const fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub const fn get(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RemoteImportCandidateRevision(u64);

impl RemoteImportCandidateRevision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RemoteImportEntryId(String);

impl RemoteImportEntryId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RemoteImportPageCursor(String);

impl RemoteImportPageCursor {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteImportState {
    Preparing,
    Ready,
    Stale,
    Failed,
    Applied,
    Discarded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteImportChangeKind {
    Added,
    Modified,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteImportBlocker {
    LedgerHeadDrift,
    IgnoreSnapshotDrift,
    LocatorBindingDrift,
    PendingOverlap,
    StagedOverlap,
    ArtifactTamper,
    RepoMembershipMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteImportProjectionOutcome {
    Pending,
    Written,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteImportSessionView {
    pub session_id: RemoteImportSessionId,
    pub state: RemoteImportState,
    pub revision: Option<RemoteImportCandidateRevision>,
    pub entry_count: u32,
    pub blockers: Vec<RemoteImportBlocker>,
    pub cleanup_pending: bool,
    pub projection_outcome: Option<RemoteImportProjectionOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteImportCandidateView {
    pub entry_id: RemoteImportEntryId,
    pub display_label: String,
    pub change_kind: RemoteImportChangeKind,
    pub blockers: Vec<RemoteImportBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteImportCandidatePage {
    pub session: RemoteImportSessionView,
    pub entries: Vec<RemoteImportCandidateView>,
    pub next_cursor: Option<RemoteImportPageCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteImportApplyReceipt {
    pub request_id: Uuid,
    pub session_id: RemoteImportSessionId,
    pub revision: RemoteImportCandidateRevision,
    pub projection_outcome: RemoteImportProjectionOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteImportRequest {
    Prepare {
        context: RemoteImportRequestContext,
        provider: RemoteProjectionProvider,
    },
    List {
        context: RemoteImportRequestContext,
    },
    Show {
        context: RemoteImportRequestContext,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    },
    Page {
        context: RemoteImportRequestContext,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        cursor: Option<RemoteImportPageCursor>,
        limit: u16,
    },
    Diff {
        context: RemoteImportRequestContext,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        entry_id: RemoteImportEntryId,
    },
    Refresh {
        context: RemoteImportRequestContext,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    },
    Apply {
        context: RemoteImportRequestContext,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    },
    Discard {
        context: RemoteImportRequestContext,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    },
}

impl RemoteImportRequest {
    pub fn context(&self) -> &RemoteImportRequestContext {
        match self {
            Self::Prepare { context, .. }
            | Self::List { context }
            | Self::Show { context, .. }
            | Self::Page { context, .. }
            | Self::Diff { context, .. }
            | Self::Refresh { context, .. }
            | Self::Apply { context, .. }
            | Self::Discard { context, .. } => context,
        }
    }

    pub const fn is_write(&self) -> bool {
        matches!(
            self,
            Self::Prepare { .. } | Self::Refresh { .. } | Self::Apply { .. } | Self::Discard { .. }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteImportResponse {
    Prepared {
        context: RemoteImportResponseContext,
        session: RemoteImportSessionView,
    },
    Sessions {
        context: RemoteImportResponseContext,
        sessions: Vec<RemoteImportSessionView>,
    },
    Session {
        context: RemoteImportResponseContext,
        session: RemoteImportSessionView,
    },
    Page {
        context: RemoteImportResponseContext,
        page: RemoteImportCandidatePage,
    },
    Diff {
        context: RemoteImportResponseContext,
        entry_id: RemoteImportEntryId,
        display_label: String,
        change_kind: RemoteImportChangeKind,
        blockers: Vec<RemoteImportBlocker>,
        projection: Arc<DiffProjection>,
    },
    Applied {
        context: RemoteImportResponseContext,
        receipt: RemoteImportApplyReceipt,
    },
    Discarded {
        context: RemoteImportResponseContext,
        session: RemoteImportSessionView,
    },
    Error {
        context: RemoteImportResponseContext,
        error: ServerError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteProjectionPushRequest {
    pub request_id: Uuid,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: ScopeNonce,
    pub provider: RemoteProjectionProvider,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteProjectionPushResponse {
    pub request_id: Uuid,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: ScopeNonce,
    pub error: Option<ServerError>,
}

#[cfg(test)]
mod tests;
