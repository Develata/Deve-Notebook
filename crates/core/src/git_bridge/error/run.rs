//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!

use super::{
    GitMirrorCommitError, GitMirrorStatusError, GitMirrorStoreError, GitPreflightError,
    GitSnapshotBootstrapError,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitMirrorRunError {
    #[error("Git mirror executor failed to inspect mirror status: {source}")]
    StatusInspect { source: GitMirrorStatusError },
    #[error("Git mirror executor failed to inspect latest Deve commit: {message}")]
    CommitList { message: String },
    #[error("Git mirror executor failed to inspect {kind} source-control changes: {message}")]
    SourceControlInspect { kind: &'static str, message: String },
    #[error("Git mirror executor failed to {action} Deve commit table: {message}")]
    CommitTable {
        action: &'static str,
        message: String,
    },
    #[error("Git mirror executor failed to load Deve commit {commit_id}: {message}")]
    CommitLoad { commit_id: String, message: String },
    #[error("Git mirror executor failed to decode Deve commit {commit_id}: {message}")]
    CommitDecode { commit_id: String, message: String },
    #[error("Git mirror executor failed to read queued Deve commit diff data: {message}")]
    CommitDiffStorage { message: String },
    #[error("Git mirror executor failed to create temporary Git mirror index: {message}")]
    TempIndex { message: String },
    #[error(transparent)]
    Store(#[from] GitMirrorStoreError),
}

impl From<GitMirrorRunError> for String {
    fn from(err: GitMirrorRunError) -> Self {
        err.to_string()
    }
}

pub(in crate::git_bridge) enum GitMirrorRunFailure {
    OutOfSync(String),
    Propagate(GitMirrorRunError),
}

impl GitMirrorRunFailure {
    pub(in crate::git_bridge) fn from_commit_error(err: GitMirrorCommitError) -> Self {
        match err {
            GitMirrorCommitError::GitPreflight(err) => Self::from_preflight_error(err),
            GitMirrorCommitError::TempIndex { message } => {
                Self::Propagate(GitMirrorRunError::TempIndex { message })
            }
            other => Self::OutOfSync(other.to_string()),
        }
    }

    pub(in crate::git_bridge) fn from_snapshot_bootstrap_error(
        err: GitSnapshotBootstrapError,
    ) -> Self {
        match err {
            GitSnapshotBootstrapError::GitPreflight(err) => Self::from_preflight_error(err),
            GitSnapshotBootstrapError::ProjectionSnapshotInspectStorage { message }
            | GitSnapshotBootstrapError::ProjectionSnapshotLoadStorage { message } => {
                Self::Propagate(GitMirrorRunError::CommitDiffStorage { message })
            }
            GitSnapshotBootstrapError::TempIndex { message } => {
                Self::Propagate(GitMirrorRunError::TempIndex { message })
            }
            other => Self::OutOfSync(other.to_string()),
        }
    }

    fn from_preflight_error(err: GitPreflightError) -> Self {
        match err {
            GitPreflightError::SourceControlInspect { kind, message } => {
                Self::Propagate(GitMirrorRunError::SourceControlInspect { kind, message })
            }
            GitPreflightError::CommitTable { action, message } => {
                Self::Propagate(GitMirrorRunError::CommitTable { action, message })
            }
            GitPreflightError::CommitLoad { commit_id, message } => {
                Self::Propagate(GitMirrorRunError::CommitLoad { commit_id, message })
            }
            GitPreflightError::CommitDecode { commit_id, message } => {
                Self::Propagate(GitMirrorRunError::CommitDecode { commit_id, message })
            }
            GitPreflightError::CommitDiffStorage { message } => {
                Self::Propagate(GitMirrorRunError::CommitDiffStorage { message })
            }
            other => Self::OutOfSync(other.to_string()),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitMirrorPushError {
    #[error("Git push mirror failed to inspect mirror status: {source}")]
    StatusInspect { source: GitMirrorStatusError },
    #[error(transparent)]
    Store(#[from] GitMirrorStoreError),
}

impl From<GitMirrorPushError> for String {
    fn from(err: GitMirrorPushError) -> Self {
        err.to_string()
    }
}
