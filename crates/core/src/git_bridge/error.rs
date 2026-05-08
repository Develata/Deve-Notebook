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
pub(super) type GitImportApplyResult<T> = std::result::Result<T, GitImportApplyError>;

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
    #[error("Git mirror snapshot bootstrap failed to inspect status: {message}")]
    StatusInspect { message: String },
    #[error("{reason}")]
    MirrorNotReady { reason: String },
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
pub(super) enum GitImportApplyError {
    #[error("failed to read imported Git worktree file {path}: {message}")]
    ReadImportedWorktreeFile { path: String, message: String },
    #[error("failed to check Git import conflict for {path}: {message}")]
    ConflictCheck { path: String, message: String },
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
        assert_eq!(
            super::GitSnapshotBootstrapError::MirrorNotReady {
                reason: "repo-local .gitignore does not ignore .notegit/".into(),
            }
            .to_string(),
            "repo-local .gitignore does not ignore .notegit/"
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
    }
}
