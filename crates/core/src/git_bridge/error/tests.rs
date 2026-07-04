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
fn git_mirror_status_error_preserves_legacy_messages() {
    assert_eq!(
        super::GitMirrorStatusError::GitignoreProtection {
            message: "permission denied".into(),
        }
        .to_string(),
        "failed to inspect .gitignore .notegit protection: permission denied"
    );
    let message: String = super::GitMirrorStatusError::GitMetadataPresence {
        message: "io error".into(),
    }
    .into();
    assert_eq!(
        message,
        "failed to inspect .git metadata presence: io error"
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
        super::GitImportPlanError::from(super::GitPreflightError::PendingSourceControlChanges {
            count: 1
        },)
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
        super::GitImportPlanError::StatusInspect {
            source: super::GitMirrorStatusError::GitMetadataPresence {
                message: "permission denied".into(),
            },
        }
        .to_string(),
        "Git import dry-run failed to inspect mirror status: failed to inspect .git metadata presence: permission denied"
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
            source: super::GitMirrorStatusError::GitignoreProtection {
                message: "permission denied".into(),
            },
        }
        .to_string(),
        "Git push mirror failed to inspect mirror status: failed to inspect .gitignore .notegit protection: permission denied"
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
            source: super::GitMirrorStatusError::GitignoreProtection {
                message: "permission denied".into(),
            },
        }
        .to_string(),
        "Git mirror executor failed to inspect mirror status: failed to inspect .gitignore .notegit protection: permission denied"
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
        super::GitMirrorRunError::TempIndex {
            message: "no space".into(),
        }
        .to_string(),
        "Git mirror executor failed to create temporary Git mirror index: no space"
    );
    assert_eq!(
        super::GitMirrorRunError::Store(super::GitMirrorStoreError::ListRecords {
            message: "table type mismatch".into(),
        })
        .to_string(),
        "failed to list Git mirror records: table type mismatch"
    );
}
