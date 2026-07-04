//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!

use super::GitCommandError;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(in crate::git_bridge) enum GitPreflightError {
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
    #[error("Git mirror is not a usable worktree: rev-parse returned {output}")]
    NotWorktree { output: String },
    #[error("Git mirror refuses to run because .notegit is already tracked by Git")]
    NotegitTracked,
    #[error("failed to inspect {kind} source-control changes: {message}")]
    SourceControlInspect { kind: &'static str, message: String },
    #[error("Git mirror refuses to run with {count} pending source-control change(s)")]
    PendingSourceControlChanges { count: usize },
    #[error("Git mirror refuses to run with {count} staged source-control change(s)")]
    StagedSourceControlChanges { count: usize },
    #[error("Git mirror refuses to push dirty Git worktree path(s): {paths}")]
    DirtyGitWorktree { paths: String },
    #[error(
        "Git mirror record ledger_seq {record_seq} does not match Deve commit ledger_seq {commit_seq}"
    )]
    MirrorRecordSeqMismatch { record_seq: u64, commit_seq: u64 },
    #[error("failed to compute queued Deve commit diff: {message}")]
    CommitDiff { message: String },
    #[error("failed to read queued Deve commit diff data: {message}")]
    CommitDiffStorage { message: String },
    #[error("failed to {action} Deve commit table: {message}")]
    CommitTable {
        action: &'static str,
        message: String,
    },
    #[error("failed to load Deve commit {commit_id}: {message}")]
    CommitLoad { commit_id: String, message: String },
    #[error("queued Git mirror record references missing Deve commit {commit_id}")]
    MissingDeveCommit { commit_id: String },
    #[error("failed to decode Deve commit {commit_id}: {message}")]
    CommitDecode { commit_id: String, message: String },
    #[error("Git mirror refuses to include path(s) outside {scope}: {paths}")]
    ProjectionScope { scope: String, paths: String },
    #[error("Git mirror terminal projection mismatch for {path}: {reason}")]
    ProjectionContentMismatch { path: String, reason: String },
}

impl From<GitPreflightError> for String {
    fn from(err: GitPreflightError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(in crate::git_bridge) enum GitReplayError {
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
    #[error("failed to read mirror .gitignore: {message}")]
    ReadGitignore { message: String },
    #[error("Git mirror refuses unsafe projection path: {path}")]
    UnsafeProjectionPath { path: String },
}

impl From<GitReplayError> for String {
    fn from(err: GitReplayError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(in crate::git_bridge) enum GitMirrorCommitError {
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
    #[error(transparent)]
    GitPreflight(#[from] GitPreflightError),
    #[error(transparent)]
    GitReplay(#[from] GitReplayError),
    #[error("failed to create temporary Git mirror index: {message}")]
    TempIndex { message: String },
    #[error("git mirror has no staged changes for queued Deve commit")]
    NoStagedChanges,
}

impl From<GitMirrorCommitError> for String {
    fn from(err: GitMirrorCommitError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(in crate::git_bridge) enum GitSnapshotBootstrapError {
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
    #[error(transparent)]
    GitPreflight(#[from] GitPreflightError),
    #[error(transparent)]
    GitReplay(#[from] GitReplayError),
    #[error("Git mirror snapshot bootstrap requires empty Git history, but HEAD is {head}")]
    NonEmptyGitHistory { head: String },
    #[error("failed to inspect current projection snapshot: {message}")]
    ProjectionSnapshotInspect { message: String },
    #[error("failed to inspect current projection snapshot data: {message}")]
    ProjectionSnapshotInspectStorage { message: String },
    #[error("failed to load current projection snapshot: {message}")]
    ProjectionSnapshotLoad { message: String },
    #[error("failed to load current projection snapshot data: {message}")]
    ProjectionSnapshotLoadStorage { message: String },
    #[error("failed to create temporary Git mirror index: {message}")]
    TempIndex { message: String },
}

impl From<GitSnapshotBootstrapError> for String {
    fn from(err: GitSnapshotBootstrapError) -> Self {
        err.to_string()
    }
}
