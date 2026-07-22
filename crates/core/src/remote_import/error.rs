//! plan_ref:
//!   - 06_backup#projection-backup-failure-modes

use super::types::{
    RemoteImportBlocker, RemoteImportCandidateRevision, RemoteImportSessionId, RemoteImportState,
};
use crate::models::RepoId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RemoteImportError {
    #[error("Remote Import storage failure: {0}")]
    Storage(String),
    #[error("Remote Import codec failure: {0}")]
    Codec(String),
    #[error("Remote Import JSON failure: {0}")]
    Json(String),
    #[error("Remote Import artifact I/O failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("Remote Import source read failure: {0}")]
    SourceRead(String),
    #[error("Remote Import artifact root is unsafe: {0}")]
    UnsafeArtifactRoot(String),
    #[error("Remote Import path {path:?} is invalid: {reason}")]
    InvalidPath { path: String, reason: String },
    #[error("Remote Import {kind} limit exceeded: limit={limit}, observed={observed}")]
    LimitExceeded {
        kind: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("Remote Import path collides case-insensitively: {0}")]
    DuplicatePath(String),
    #[error("Remote Import already has active session {0}")]
    ActiveSession(RemoteImportSessionId),
    #[error("Remote Import session {0} was not found")]
    SessionNotFound(RemoteImportSessionId),
    #[error("Remote Import session {session_id} is in state {state:?}, expected {expected}")]
    InvalidState {
        session_id: RemoteImportSessionId,
        state: RemoteImportState,
        expected: &'static str,
    },
    #[error("Remote Import session {0} generation is stale")]
    StaleGeneration(RemoteImportSessionId),
    #[error("Remote Import artifact verification failed: {0}")]
    ArtifactTampered(String),
    #[error("Remote Import candidate revision overflow")]
    RevisionOverflow,
    #[error("Remote Import candidate revision {revision:?} was published by another refresh")]
    CandidateRevisionConflict {
        revision: RemoteImportCandidateRevision,
    },
    #[error("Remote Import session {session_id} is stale: {blockers:?}")]
    Stale {
        session_id: RemoteImportSessionId,
        blockers: Vec<RemoteImportBlocker>,
    },
    #[error("Remote Import session {session_id} is blocked: {blockers:?}")]
    Blocked {
        session_id: RemoteImportSessionId,
        blockers: Vec<RemoteImportBlocker>,
    },
    #[error("Remote Import authority apply failed: {0}")]
    ApplyFailed(String),
    #[error("Remote Import repair inventory changed after dry-run")]
    RepairPlanChanged,
    #[error("Remote Import removal state changed for repo {0}")]
    RepoRemovalChanged(RepoId),
}

impl RemoteImportError {
    pub(crate) fn storage(error: impl std::fmt::Display) -> Self {
        Self::Storage(error.to_string())
    }

    pub(super) fn codec(error: impl std::fmt::Display) -> Self {
        Self::Codec(error.to_string())
    }

    pub(crate) fn json(error: impl std::fmt::Display) -> Self {
        Self::Json(error.to_string())
    }

    pub(super) fn source_read(error: impl std::fmt::Display) -> Self {
        Self::SourceRead(error.to_string())
    }

    pub(crate) fn apply_failed(error: impl std::fmt::Display) -> Self {
        Self::ApplyFailed(error.to_string())
    }
}

pub type RemoteImportResult<T> = Result<T, RemoteImportError>;
