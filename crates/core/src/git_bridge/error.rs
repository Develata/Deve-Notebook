//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle

pub(super) type GitBridgeResult<T> = std::result::Result<T, GitBridgeError>;
pub(super) type GitCommandResult<T> = std::result::Result<T, GitCommandError>;
pub(super) type GitPreflightResult<T> = std::result::Result<T, GitPreflightError>;
pub(super) type GitReplayResult<T> = std::result::Result<T, GitReplayError>;
pub(super) type GitReplayPlanResult<T> = std::result::Result<T, GitReplayPlanError>;
pub(super) type GitProjectionReplayResult<T> = std::result::Result<T, GitProjectionReplayError>;
pub(super) type GitMirrorCommitResult<T> = std::result::Result<T, GitMirrorCommitError>;
pub(super) type GitSnapshotBootstrapResult<T> = std::result::Result<T, GitSnapshotBootstrapError>;
pub type GitImportPlanResult<T> = std::result::Result<T, GitImportPlanError>;
pub type GitImportApplyResult<T> = std::result::Result<T, GitImportApplyError>;
pub type GitMirrorRunResult<T> = std::result::Result<T, GitMirrorRunError>;
pub type GitMirrorPushResult<T> = std::result::Result<T, GitMirrorPushError>;
pub type GitMirrorStoreResult<T> = std::result::Result<T, GitMirrorStoreError>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum GitBridgeError {
    #[error("Git push mirror refuses invalid {label}: {value:?}")]
    InvalidPushName { label: &'static str, value: String },
    #[error("Git push mirror requires a named branch; detached HEAD needs --branch")]
    DetachedHead,
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum GitCommandError {
    #[error("failed to run git {args}: {message}")]
    Spawn { args: String, message: String },
    #[error("failed to open stdin for git {args}")]
    MissingStdin { args: String },
    #[error("failed to write stdin for git {args}: {message}")]
    StdinWrite { args: String, message: String },
    #[error("failed to wait for git {args}: {message}")]
    Wait { args: String, message: String },
    #[error("git {args} returned non-UTF-8 field: {message}")]
    NonUtf8Field { args: String, message: String },
    #[error("git {args} failed with status {status}")]
    Status { args: String, status: String },
    #[error("git {args} failed (status {status}): {detail}")]
    StatusDetail {
        args: String,
        status: String,
        detail: String,
    },
}

impl GitCommandError {
    pub(super) fn status(args: &[&str], output: &std::process::Output) -> Self {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        let args = args.join(" ");
        let status = output.status.to_string();
        if detail.is_empty() {
            return Self::Status { args, status };
        }
        Self::StatusDetail {
            args,
            status,
            detail,
        }
    }
}

impl From<GitCommandError> for String {
    fn from(err: GitCommandError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum GitPreflightError {
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
pub(super) enum GitReplayError {
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
pub(super) enum GitReplayPlanError {
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
pub(super) enum GitProjectionReplayError {
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
pub(super) enum GitMirrorCommitError {
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
pub(super) enum GitSnapshotBootstrapError {
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
    #[error("failed to load current projection snapshot: {message}")]
    ProjectionSnapshotLoad { message: String },
    #[error("failed to create temporary Git mirror index: {message}")]
    TempIndex { message: String },
}

impl From<GitSnapshotBootstrapError> for String {
    fn from(err: GitSnapshotBootstrapError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitImportPlanError {
    #[error("{message}")]
    GitCommand { message: String },
    #[error("{message}")]
    GitPreflight { message: String },
    #[error("Git import dry-run failed to inspect mirror status: {message}")]
    StatusInspect { message: String },
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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitMirrorRunError {
    #[error("Git mirror executor failed to inspect mirror status: {message}")]
    StatusInspect { message: String },
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
    #[error("Git mirror executor failed to read parent Git mirror record {parent_id}: {message}")]
    ParentRecordRead { parent_id: String, message: String },
    #[error(transparent)]
    Store(#[from] GitMirrorStoreError),
}

impl From<GitMirrorRunError> for String {
    fn from(err: GitMirrorRunError) -> Self {
        err.to_string()
    }
}

pub(super) enum GitMirrorRunFailure {
    OutOfSync(String),
    Propagate(GitMirrorRunError),
}

impl GitMirrorRunFailure {
    pub(super) fn from_commit_error(err: GitMirrorCommitError) -> Self {
        match err {
            GitMirrorCommitError::GitPreflight(err) => Self::from_preflight_error(err),
            other => Self::OutOfSync(other.to_string()),
        }
    }

    pub(super) fn from_replay_plan_error(err: GitReplayPlanError) -> Self {
        match err {
            GitReplayPlanError::GitPreflight(err) => Self::from_preflight_error(err),
            GitReplayPlanError::ParentRecordRead { parent_id, message } => {
                Self::Propagate(GitMirrorRunError::ParentRecordRead { parent_id, message })
            }
            other => Self::OutOfSync(other.to_string()),
        }
    }

    pub(super) fn from_projection_replay_error(err: GitProjectionReplayError) -> Self {
        match err {
            GitProjectionReplayError::ProjectionDiffStorage { message, .. } => {
                Self::Propagate(GitMirrorRunError::CommitDiffStorage { message })
            }
            other => Self::OutOfSync(format!("Git mirror projection replay failed: {other}")),
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
    #[error("Git push mirror failed to inspect mirror status: {message}")]
    StatusInspect { message: String },
    #[error(transparent)]
    Store(#[from] GitMirrorStoreError),
}

impl From<GitMirrorPushError> for String {
    fn from(err: GitMirrorPushError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitMirrorStoreError {
    #[error("failed to initialize Git mirror store: {message}")]
    Init { message: String },
    #[error("failed to read Git mirror record {deve_commit_id}: {message}")]
    ReadRecord {
        deve_commit_id: String,
        message: String,
    },
    #[error("failed to list Git mirror records: {message}")]
    ListRecords { message: String },
    #[error("failed to decode Git mirror record {deve_commit_id}: {message}")]
    DecodeRecord {
        deve_commit_id: String,
        message: String,
    },
    #[error("failed to encode Git mirror record {deve_commit_id}: {message}")]
    EncodeRecord {
        deve_commit_id: String,
        message: String,
    },
    #[error("failed to write Git mirror record {deve_commit_id}: {message}")]
    WriteRecord {
        deve_commit_id: String,
        message: String,
    },
    #[error("Git mirror record not found for Deve commit {deve_commit_id}")]
    MissingRecord { deve_commit_id: String },
}

impl From<GitMirrorStoreError> for String {
    fn from(err: GitMirrorStoreError) -> Self {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::GitCommandError;

    #[test]
    fn git_command_error_preserves_legacy_status_text() {
        assert_eq!(
            GitCommandError::Status {
                args: "status".into(),
                status: "exit status: 1".into(),
            }
            .to_string(),
            "git status failed with status exit status: 1"
        );
        assert_eq!(
            GitCommandError::StatusDetail {
                args: "push origin main".into(),
                status: "exit status: 128".into(),
                detail: "fatal: rejected".into(),
            }
            .to_string(),
            "git push origin main failed (status exit status: 128): fatal: rejected"
        );
    }

    #[test]
    fn git_command_error_converts_to_string_for_legacy_callers() {
        let message: String = GitCommandError::Spawn {
            args: "rev-parse HEAD".into(),
            message: "No such file or directory".into(),
        }
        .into();

        assert_eq!(
            message,
            "failed to run git rev-parse HEAD: No such file or directory"
        );
    }

    #[test]
    fn git_preflight_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitPreflightError::ProjectionScope {
                scope: "queued Deve commit".into(),
                paths: "extra.md".into(),
            }
            .to_string(),
            "Git mirror refuses to include path(s) outside queued Deve commit: extra.md"
        );
        assert_eq!(
            super::GitPreflightError::PendingSourceControlChanges { count: 2 }.to_string(),
            "Git mirror refuses to run with 2 pending source-control change(s)"
        );
    }

    #[test]
    fn git_replay_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitReplayError::UnsafeProjectionPath {
                path: ".notegit/state".into(),
            }
            .to_string(),
            "Git mirror refuses unsafe projection path: .notegit/state"
        );
        assert_eq!(
            super::GitReplayError::ReadGitignore {
                message: "missing".into(),
            }
            .to_string(),
            "failed to read mirror .gitignore: missing"
        );
    }

    #[test]
    fn git_replay_plan_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitReplayPlanError::NonContiguousCommitChain {
                commit_id: "c2".into(),
                parent: Some("old".into()),
                expected: "c1".into(),
            }
            .to_string(),
            "queued Git mirror records are not a contiguous Deve commit chain: c2 parent is Some(\"old\"), expected c1"
        );
        assert_eq!(
            super::GitReplayPlanError::HeadMismatch {
                parent_id: "p1".into(),
                head: Some("abc".into()),
                expected: "def".into(),
            }
            .to_string(),
            "Git HEAD does not match mirrored parent p1: head=Some(\"abc\") expected=def"
        );
    }

    #[test]
    fn git_projection_replay_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitProjectionReplayError::ProjectionDiff {
                commit_id: "c1".into(),
                message: "table missing".into(),
            }
            .to_string(),
            "failed to compute projection diff for c1: table missing"
        );
        assert_eq!(
            super::GitProjectionReplayError::EmptyProjectionDiff {
                commit_id: "c1".into(),
            }
            .to_string(),
            "Deve commit c1 has no projection diff to mirror"
        );
    }

    #[test]
    fn git_mirror_commit_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitMirrorCommitError::NoStagedChanges.to_string(),
            "git mirror has no staged changes for queued Deve commit"
        );
        assert_eq!(
            super::GitMirrorCommitError::GitPreflight(
                super::GitPreflightError::PendingSourceControlChanges { count: 3 },
            )
            .to_string(),
            "Git mirror refuses to run with 3 pending source-control change(s)"
        );
    }

    #[test]
    fn git_snapshot_bootstrap_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitSnapshotBootstrapError::NonEmptyGitHistory { head: "abc".into() }.to_string(),
            "Git mirror snapshot bootstrap requires empty Git history, but HEAD is abc"
        );
        assert_eq!(
            super::GitSnapshotBootstrapError::ProjectionSnapshotLoad {
                message: "missing table".into(),
            }
            .to_string(),
            "failed to load current projection snapshot: missing table"
        );
    }

    #[test]
    fn git_import_apply_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitImportApplyError::ReadImportedWorktreeFile {
                path: "note.md".into(),
                message: "missing".into(),
            }
            .to_string(),
            "failed to read imported Git worktree file note.md: missing"
        );
        assert_eq!(
            super::GitImportApplyError::MissingTrackedDoc {
                status: "modified",
                path: "note.md".into(),
            }
            .to_string(),
            "Git import requires tracked Deve doc for modified path: note.md"
        );
        assert_eq!(
            super::GitImportApplyError::RenameTargetAlreadyTracked {
                path: "moved.md".into(),
            }
            .to_string(),
            "Git import rename target is already tracked by another Deve doc: moved.md"
        );
        assert_eq!(
            super::GitImportApplyError::PendingEntryInspect {
                path: "note.md".into(),
                message: "table missing".into(),
            }
            .to_string(),
            "failed to inspect existing Git import pending entry for note.md: table missing"
        );
        assert_eq!(
            super::GitImportApplyError::PendingEntryWrite {
                message: "transaction failed".into(),
            }
            .to_string(),
            "failed to write Git import pending entries: transaction failed"
        );
        assert_eq!(
            super::GitImportApplyError::Plan(super::GitImportPlanError::MissingHead).to_string(),
            "Git import dry-run requires Git HEAD"
        );
        assert_eq!(
            super::GitImportApplyError::LocalRepoApply {
                repo_name: "default".into(),
                message: "database unavailable".into(),
            }
            .to_string(),
            "Git import apply failed while writing pending entries for repo default: database unavailable"
        );
    }

    #[test]
    fn git_import_plan_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitImportPlanError::from(super::GitCommandError::Spawn {
                args: "status".into(),
                message: "missing git".into(),
            })
            .to_string(),
            "failed to run git status: missing git"
        );
        assert_eq!(
            super::GitImportPlanError::from(
                super::GitPreflightError::PendingSourceControlChanges { count: 1 },
            )
            .to_string(),
            "Git mirror refuses to run with 1 pending source-control change(s)"
        );
        assert_eq!(
            super::GitImportPlanError::MissingHead.to_string(),
            "Git import dry-run requires Git HEAD"
        );
        assert_eq!(
            super::GitImportPlanError::MirrorNotReady {
                reason: "state=disabled git=missing".into(),
            }
            .to_string(),
            "Git import dry-run requires ready Git mirror: state=disabled git=missing"
        );
        assert_eq!(
            super::GitImportPlanError::UnsafePath {
                path: "../note.md".into(),
            }
            .to_string(),
            "Git import refuses unsafe path: ../note.md"
        );
    }

    #[test]
    fn git_mirror_push_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitMirrorPushError::StatusInspect {
                message: "permission denied".into(),
            }
            .to_string(),
            "Git push mirror failed to inspect mirror status: permission denied"
        );
        assert_eq!(
            super::GitMirrorPushError::Store(super::GitMirrorStoreError::ListRecords {
                message: "table type mismatch".into(),
            })
            .to_string(),
            "failed to list Git mirror records: table type mismatch"
        );
    }

    #[test]
    fn git_mirror_run_error_preserves_legacy_messages() {
        assert_eq!(
            super::GitMirrorRunError::StatusInspect {
                message: "permission denied".into(),
            }
            .to_string(),
            "Git mirror executor failed to inspect mirror status: permission denied"
        );
        assert_eq!(
            super::GitMirrorRunError::CommitList {
                message: "table missing".into(),
            }
            .to_string(),
            "Git mirror executor failed to inspect latest Deve commit: table missing"
        );
        assert_eq!(
            super::GitMirrorRunError::SourceControlInspect {
                kind: "pending",
                message: "table missing".into(),
            }
            .to_string(),
            "Git mirror executor failed to inspect pending source-control changes: table missing"
        );
        assert_eq!(
            super::GitMirrorRunError::CommitTable {
                action: "open",
                message: "table missing".into(),
            }
            .to_string(),
            "Git mirror executor failed to open Deve commit table: table missing"
        );
        assert_eq!(
            super::GitMirrorRunError::CommitDiffStorage {
                message: "range table missing".into(),
            }
            .to_string(),
            "Git mirror executor failed to read queued Deve commit diff data: range table missing"
        );
        assert_eq!(
            super::GitMirrorRunError::Store(super::GitMirrorStoreError::ListRecords {
                message: "table type mismatch".into(),
            })
            .to_string(),
            "failed to list Git mirror records: table type mismatch"
        );
    }

    #[test]
    fn git_mirror_run_failure_classifies_infra_and_business_failures() {
        match super::GitMirrorRunFailure::from_commit_error(
            super::GitMirrorCommitError::GitPreflight(super::GitPreflightError::CommitDiff {
                message: "lost projected path".into(),
            }),
        ) {
            super::GitMirrorRunFailure::OutOfSync(reason) => {
                assert!(
                    reason.contains("failed to compute queued Deve commit diff"),
                    "{reason}"
                );
            }
            super::GitMirrorRunFailure::Propagate(err) => {
                panic!("CommitDiff should remain mirror out-of-sync, got {err:?}");
            }
        }

        match super::GitMirrorRunFailure::from_commit_error(
            super::GitMirrorCommitError::GitPreflight(super::GitPreflightError::CommitTable {
                action: "open",
                message: "table missing".into(),
            }),
        ) {
            super::GitMirrorRunFailure::Propagate(super::GitMirrorRunError::CommitTable {
                action: "open",
                message,
            }) => assert_eq!(message, "table missing"),
            other => panic!(
                "CommitTable should propagate, got {}",
                classify_failure(other)
            ),
        }

        match super::GitMirrorRunFailure::from_commit_error(
            super::GitMirrorCommitError::GitPreflight(
                super::GitPreflightError::CommitDiffStorage {
                    message: "range table missing".into(),
                },
            ),
        ) {
            super::GitMirrorRunFailure::Propagate(
                super::GitMirrorRunError::CommitDiffStorage { message },
            ) => assert_eq!(message, "range table missing"),
            other => panic!(
                "CommitDiffStorage should propagate, got {}",
                classify_failure(other)
            ),
        }

        match super::GitMirrorRunFailure::from_commit_error(
            super::GitMirrorCommitError::GitPreflight(
                super::GitPreflightError::SourceControlInspect {
                    kind: "pending",
                    message: "table missing".into(),
                },
            ),
        ) {
            super::GitMirrorRunFailure::Propagate(
                super::GitMirrorRunError::SourceControlInspect {
                    kind: "pending",
                    message,
                },
            ) => assert_eq!(message, "table missing"),
            other => panic!(
                "SourceControlInspect should propagate, got {}",
                classify_failure(other)
            ),
        }

        match super::GitMirrorRunFailure::from_replay_plan_error(
            super::GitReplayPlanError::ParentRecordRead {
                parent_id: "p1".into(),
                message: "store failed".into(),
            },
        ) {
            super::GitMirrorRunFailure::Propagate(super::GitMirrorRunError::ParentRecordRead {
                parent_id,
                message,
            }) => {
                assert_eq!(parent_id, "p1");
                assert_eq!(message, "store failed");
            }
            other => panic!(
                "ParentRecordRead should propagate, got {}",
                classify_failure(other)
            ),
        }

        match super::GitMirrorRunFailure::from_replay_plan_error(
            super::GitReplayPlanError::NonContiguousCommitChain {
                commit_id: "c2".into(),
                parent: Some("old".into()),
                expected: "c1".into(),
            },
        ) {
            super::GitMirrorRunFailure::OutOfSync(reason) => {
                assert!(
                    reason.contains("not a contiguous Deve commit chain"),
                    "{reason}"
                );
            }
            super::GitMirrorRunFailure::Propagate(err) => {
                panic!("NonContiguousCommitChain should be out-of-sync, got {err:?}");
            }
        }

        match super::GitMirrorRunFailure::from_projection_replay_error(
            super::GitProjectionReplayError::ProjectionDiffStorage {
                commit_id: "c1".into(),
                message: "range table missing".into(),
            },
        ) {
            super::GitMirrorRunFailure::Propagate(
                super::GitMirrorRunError::CommitDiffStorage { message },
            ) => assert_eq!(message, "range table missing"),
            other => panic!(
                "ProjectionDiffStorage should propagate, got {}",
                classify_failure(other)
            ),
        }

        match super::GitMirrorRunFailure::from_projection_replay_error(
            super::GitProjectionReplayError::ProjectionDiff {
                commit_id: "c1".into(),
                message: "lost projected path".into(),
            },
        ) {
            super::GitMirrorRunFailure::OutOfSync(reason) => {
                assert!(
                    reason.contains("Git mirror projection replay failed"),
                    "{reason}"
                );
                assert!(reason.contains("lost projected path"), "{reason}");
            }
            super::GitMirrorRunFailure::Propagate(err) => {
                panic!("ProjectionDiff should remain mirror out-of-sync, got {err:?}");
            }
        }
    }

    fn classify_failure(failure: super::GitMirrorRunFailure) -> String {
        match failure {
            super::GitMirrorRunFailure::OutOfSync(reason) => format!("out_of_sync: {reason}"),
            super::GitMirrorRunFailure::Propagate(err) => format!("propagate: {err:?}"),
        }
    }
}
