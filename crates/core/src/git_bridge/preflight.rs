//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Shared preflight checks and Deve commit lookup for Git mirror execution.

use super::error::{GitPreflightError, GitPreflightResult};
use super::git_cmd;
use super::store::GitMirrorRecord;
use crate::source_control::{self, CommitInfo};
use crate::utils::{notegit, path::to_forward_slash};
use redb::Database;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn ensure_git_worktree(repo_root: &Path) -> GitPreflightResult<()> {
    let inside = git_cmd::run(repo_root, &["rev-parse", "--is-inside-work-tree"])?;
    if inside.trim() == "true" {
        return Ok(());
    }
    Err(GitPreflightError::NotWorktree {
        output: inside.trim().to_string(),
    })
}

pub(super) fn ensure_notegit_is_not_tracked(repo_root: &Path) -> GitPreflightResult<()> {
    let paths = git_cmd::run_z_paths(repo_root, &["ls-files", "-z", "--", notegit::NOTE_GIT_DIR])?;
    if paths.is_empty() {
        return Ok(());
    }
    Err(GitPreflightError::NotegitTracked)
}

pub(super) fn ensure_source_control_clean(db: &Database) -> GitPreflightResult<()> {
    let pending = source_control::pending_fs::list_all(db).map_err(|err| {
        GitPreflightError::SourceControlInspect {
            kind: "pending",
            message: err.to_string(),
        }
    })?;
    if !pending.is_empty() {
        return Err(GitPreflightError::PendingSourceControlChanges {
            count: pending.len(),
        });
    }

    let staged = source_control::staging::list_staged_entries(db).map_err(|err| {
        GitPreflightError::SourceControlInspect {
            kind: "staged",
            message: err.to_string(),
        }
    })?;
    if !staged.is_empty() {
        return Err(GitPreflightError::StagedSourceControlChanges {
            count: staged.len(),
        });
    }
    Ok(())
}

pub(super) fn ensure_git_worktree_clean(repo_root: &Path) -> GitPreflightResult<()> {
    let changed = git_changed_paths(repo_root)?;
    if changed.is_empty() {
        return Ok(());
    }
    Err(GitPreflightError::DirtyGitWorktree {
        paths: changed.into_iter().collect::<Vec<_>>().join(", "),
    })
}

pub(super) fn ensure_git_changes_match_deve_commit(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> GitPreflightResult<()> {
    let expected = expected_mirror_paths(db, record)?;
    ensure_git_changes_within(repo_root, expected, "queued Deve commit")
}

pub(super) fn ensure_git_changes_match_deve_commits(
    db: &Database,
    repo_root: &Path,
    records: &[GitMirrorRecord],
) -> GitPreflightResult<()> {
    let mut expected = BTreeSet::new();
    for record in records {
        expected.extend(expected_mirror_paths(db, record)?);
    }
    ensure_git_changes_within(repo_root, expected, "queued Deve commits")
}

pub(super) fn ensure_git_changes_match_snapshot_paths(
    repo_root: &Path,
    paths: impl IntoIterator<Item = String>,
) -> GitPreflightResult<()> {
    let mut expected = BTreeSet::new();
    expected.insert(".gitignore".to_string());
    expected.extend(paths.into_iter().map(|path| to_forward_slash(&path)));
    ensure_git_changes_within(repo_root, expected, "current Deve projection snapshot")
}

pub(super) fn expected_mirror_paths(
    db: &Database,
    record: &GitMirrorRecord,
) -> GitPreflightResult<BTreeSet<String>> {
    let commit = load_deve_commit(db, &record.deve_commit_id)?;
    if commit.ledger_seq != record.ledger_seq {
        return Err(GitPreflightError::MirrorRecordSeqMismatch {
            record_seq: record.ledger_seq,
            commit_seq: commit.ledger_seq,
        });
    }

    let diffs = match source_control::commit_diff::compare_commits_checked(
        db,
        commit.parent_id.as_deref(),
        &commit.id,
    ) {
        Ok(diffs) => diffs,
        Err(err) => return Err(map_commit_diff_error(err)),
    };
    let mut paths = BTreeSet::new();
    paths.insert(".gitignore".to_string());
    for diff in diffs {
        paths.insert(to_forward_slash(&diff.path));
        if let Some(previous_path) = diff.previous_path {
            paths.insert(to_forward_slash(&previous_path));
        }
    }
    Ok(paths)
}

pub(super) fn map_commit_diff_error(err: source_control::CommitDiffError) -> GitPreflightError {
    match err {
        source_control::CommitDiffError::CommitTable { action, message } => {
            GitPreflightError::CommitTable { action, message }
        }
        source_control::CommitDiffError::CommitLoad { commit_id, message } => {
            GitPreflightError::CommitLoad { commit_id, message }
        }
        source_control::CommitDiffError::CommitDecode { commit_id, message } => {
            GitPreflightError::CommitDecode { commit_id, message }
        }
        source_control::CommitDiffError::CommitNotFound { commit_id } => {
            GitPreflightError::MissingDeveCommit { commit_id }
        }
        source_control::CommitDiffError::LedgerRange { .. }
        | source_control::CommitDiffError::ContentLoad { .. } => {
            GitPreflightError::CommitDiffStorage {
                message: err.to_string(),
            }
        }
        other => GitPreflightError::CommitDiff {
            message: other.to_string(),
        },
    }
}

pub(super) fn load_deve_commit(db: &Database, commit_id: &str) -> GitPreflightResult<CommitInfo> {
    let read_txn = db
        .begin_read()
        .map_err(|err| GitPreflightError::CommitTable {
            action: "read",
            message: err.to_string(),
        })?;
    let table = read_txn
        .open_table(source_control::commits::COMMITS_TABLE)
        .map_err(|err| GitPreflightError::CommitTable {
            action: "open",
            message: err.to_string(),
        })?;
    let raw = table
        .get(commit_id)
        .map_err(|err| GitPreflightError::CommitLoad {
            commit_id: commit_id.to_string(),
            message: err.to_string(),
        })?
        .ok_or_else(|| GitPreflightError::MissingDeveCommit {
            commit_id: commit_id.to_string(),
        })?;
    serde_json::from_str(raw.value()).map_err(|err| GitPreflightError::CommitDecode {
        commit_id: commit_id.to_string(),
        message: err.to_string(),
    })
}

fn ensure_git_changes_within(
    repo_root: &Path,
    expected: BTreeSet<String>,
    scope: &str,
) -> GitPreflightResult<()> {
    let changed = git_changed_paths(repo_root)?;
    let unexpected: Vec<_> = changed
        .into_iter()
        .filter(|path| notegit::is_internal_repo_path(path) || !expected.contains(path))
        .collect();
    if unexpected.is_empty() {
        return Ok(());
    }
    Err(GitPreflightError::ProjectionScope {
        scope: scope.to_string(),
        paths: unexpected.join(", "),
    })
}

fn git_changed_paths(repo_root: &Path) -> GitPreflightResult<BTreeSet<String>> {
    let mut paths = BTreeSet::new();
    for args in [
        &["diff", "--name-only", "-z"][..],
        &["diff", "--cached", "--name-only", "-z"][..],
        &["ls-files", "-o", "--exclude-standard", "-z"][..],
    ] {
        paths.extend(git_cmd::run_z_paths(repo_root, args)?);
    }
    Ok(paths)
}
