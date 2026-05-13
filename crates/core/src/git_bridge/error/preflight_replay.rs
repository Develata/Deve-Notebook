//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle
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
pub(in crate::git_bridge) enum GitReplayPlanError {
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
    #[error(transparent)]
    GitPreflight(#[from] GitPreflightError),
    #[error(transparent)]
    GitReplay(#[from] GitReplayError),
    #[error(
        "Git mirror record ledger_seq {record_seq} does not match Deve commit ledger_seq {commit_seq}"
    )]
    MirrorRecordSeqMismatch { record_seq: u64, commit_seq: u64 },
    #[error(
        "queued Git mirror records are not a contiguous Deve commit chain: {commit_id} parent is {parent:?}, expected {expected}"
    )]
    NonContiguousCommitChain {
        commit_id: String,
        parent: Option<String>,
        expected: String,
    },
    #[error("failed to read parent Git mirror record {parent_id}: {message}")]
    ParentRecordRead { parent_id: String, message: String },
    #[error("first queued Git mirror commit parent {parent_id} is not mirrored")]
    ParentNotMirrored { parent_id: String },
    #[error("first queued Git mirror commit parent {parent_id} is {state}")]
    ParentStateNotCommitted { parent_id: String, state: String },
    #[error("committed parent Git mirror record {parent_id} has no git_commit_id")]
    ParentMissingGitCommit { parent_id: String },
    #[error(
        "Git HEAD does not match mirrored parent {parent_id}: head={head:?} expected={expected}"
    )]
    HeadMismatch {
        parent_id: String,
        head: Option<String>,
        expected: String,
    },
}

impl From<GitReplayPlanError> for String {
    fn from(err: GitReplayPlanError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(in crate::git_bridge) enum GitProjectionReplayError {
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
    #[error(transparent)]
    GitReplay(#[from] GitReplayError),
    #[error("failed to compute projection diff for {commit_id}: {message}")]
    ProjectionDiff { commit_id: String, message: String },
    #[error("failed to read projection diff data for {commit_id}: {message}")]
    ProjectionDiffStorage { commit_id: String, message: String },
    #[error("Deve commit {commit_id} has no projection diff to mirror")]
    EmptyProjectionDiff { commit_id: String },
}

impl From<GitProjectionReplayError> for String {
    fn from(err: GitProjectionReplayError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(in crate::git_bridge) enum GitMirrorCommitError {
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
    #[error(transparent)]
    GitPreflight(#[from] GitPreflightError),
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
