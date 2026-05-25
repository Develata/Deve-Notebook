//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 14_commands#cli-commands
//!

use super::{GitCommandError, GitMirrorStatusError, GitPreflightError};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitImportPlanError {
    #[error("{message}")]
    GitCommand { message: String },
    #[error("{message}")]
    GitPreflight { message: String },
    #[error("Git import dry-run failed to inspect mirror status: {source}")]
    StatusInspect { source: GitMirrorStatusError },
    #[error("Git import dry-run requires ready Git mirror: {reason}")]
    MirrorNotReady { reason: String },
    #[error("Git import dry-run requires Git HEAD")]
    MissingHead,
    #[error("Git import refuses unsafe path: {path}")]
    UnsafePath { path: String },
}

impl From<GitCommandError> for GitImportPlanError {
    fn from(err: GitCommandError) -> Self {
        Self::GitCommand {
            message: err.to_string(),
        }
    }
}

impl From<GitPreflightError> for GitImportPlanError {
    fn from(err: GitPreflightError) -> Self {
        Self::GitPreflight {
            message: err.to_string(),
        }
    }
}

impl From<GitImportPlanError> for String {
    fn from(err: GitImportPlanError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitImportApplyError {
    #[error(transparent)]
    Plan(#[from] GitImportPlanError),
    #[error(
        "Git import apply failed while writing pending entries for repo {repo_name}: {message}"
    )]
    LocalRepoApply { repo_name: String, message: String },
    #[error("failed to read imported Git worktree file {path}: {message}")]
    ReadImportedWorktreeFile { path: String, message: String },
    #[error("failed to check Git import conflict for {path}: {message}")]
    ConflictCheck { path: String, message: String },
    #[error("failed to inspect Git import staged changes: {message}")]
    StagedInspect { message: String },
    #[error("failed to inspect existing Git import pending entry for {path}: {message}")]
    PendingEntryInspect { path: String, message: String },
    #[error("failed to write Git import pending entries: {message}")]
    PendingEntryWrite { message: String },
    #[error("failed to inspect tracked path {path}: {message}")]
    TrackedPathInspect { path: String, message: String },
    #[error("Git import refuses added path already tracked by Deve: {path}")]
    AddedPathAlreadyTracked { path: String },
    #[error("Git import requires tracked Deve doc for {status} path: {path}")]
    MissingTrackedDoc { status: &'static str, path: String },
    #[error("Git import rename is missing previous path: {path}")]
    RenameMissingPreviousPath { path: String },
    #[error("Git import requires tracked Deve doc for renamed path: {previous_path}")]
    RenameMissingTrackedDoc { previous_path: String },
    #[error("Git import rename target is already tracked by another Deve doc: {path}")]
    RenameTargetAlreadyTracked { path: String },
}

impl From<GitImportApplyError> for String {
    fn from(err: GitImportApplyError) -> Self {
        err.to_string()
    }
}
