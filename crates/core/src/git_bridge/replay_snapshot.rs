//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Snapshot bootstrap for an empty Git mirror history.

use super::error::{
    GitMirrorRunError, GitMirrorRunFailure, GitMirrorRunResult, GitSnapshotBootstrapError,
    GitSnapshotBootstrapResult,
};
use super::executor::{GitMirrorRunReport, commit_message};
use super::git_cmd;
use super::preflight::{
    ensure_git_changes_match_snapshot_paths, ensure_git_worktree, ensure_notegit_is_not_tracked,
    ensure_source_control_clean,
};
use super::replay_git::{
    add_blob_to_index, add_gitignore_to_index, commit_tree, read_parent_tree,
    sync_main_index_to_head, update_head,
};
use super::status::{GitMirrorState, inspect_repo_root};
use super::store::{GitMirrorRecord, mark_committed, mark_out_of_sync, queue_deve_commit};
use crate::models::RepoId;
use crate::source_control::{self, CommitInfo};
use redb::Database;
use std::path::Path;

pub(super) fn run_snapshot_bootstrap(
    db: &Database,
    repo_root: &Path,
    repo_id: RepoId,
) -> GitMirrorRunResult<GitMirrorRunReport> {
    let Some(commit) = source_control::commits::list(db, 1)
        .map_err(|err| GitMirrorRunError::CommitList {
            message: err.to_string(),
        })?
        .into_iter()
        .next()
    else {
        return Ok(GitMirrorRunReport::default());
    };
    let status = inspect_repo_root(repo_root)
        .map_err(|source| GitMirrorRunError::StatusInspect { source })?;
    let record = queue_deve_commit(db, repo_id, &commit)?;
    let mut report = GitMirrorRunReport {
        attempted: 1,
        ..GitMirrorRunReport::default()
    };
    if status.state != GitMirrorState::Ready {
        let reason = status.reason.unwrap_or_else(|| {
            format!(
                "Git mirror is not ready: state={} git={}",
                status.state.as_str(),
                status.git_metadata_kind.as_str()
            )
        });
        let updated = mark_out_of_sync(db, &record.deve_commit_id, reason)?;
        report.out_of_sync = 1;
        report.records.push(updated);
        return Ok(report);
    }

    match create_git_commit_from_snapshot(db, repo_root, &record, &commit) {
        Ok(git_commit_id) => {
            let updated = mark_committed(db, &record.deve_commit_id, &git_commit_id)?;
            report.committed = 1;
            report.records.push(updated);
        }
        Err(reason) => {
            let reason = match GitMirrorRunFailure::from_snapshot_bootstrap_error(reason) {
                GitMirrorRunFailure::OutOfSync(reason) => reason,
                GitMirrorRunFailure::Propagate(err) => return Err(err),
            };
            let updated = mark_out_of_sync(db, &record.deve_commit_id, reason)?;
            report.out_of_sync = 1;
            report.records.push(updated);
        }
    }
    Ok(report)
}

fn create_git_commit_from_snapshot(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
    commit: &CommitInfo,
) -> GitSnapshotBootstrapResult<String> {
    preflight_snapshot_bootstrap(db, repo_root, commit)?;
    let temp_dir = tempfile::tempdir().map_err(|err| GitSnapshotBootstrapError::TempIndex {
        message: err.to_string(),
    })?;
    let index_path = temp_dir.path().join("mirror-bootstrap.index");
    read_parent_tree(repo_root, &index_path, None)?;
    let files = source_control::commit_diff::projection_files_at_commit(db, &commit.id)
        .map_err(map_projection_snapshot_load_error)?;
    for file in &files {
        add_blob_to_index(
            repo_root,
            &index_path,
            &file.path,
            file.new_content.as_bytes(),
        )?;
    }
    add_gitignore_to_index(repo_root, &index_path)?;
    let tree = git_cmd::run_env(
        repo_root,
        &["write-tree"],
        &[("GIT_INDEX_FILE", &index_path)],
    )?
    .trim()
    .to_string();
    let git_commit = commit_tree(repo_root, &tree, None, &commit_message(record))?;
    update_head(repo_root, &git_commit, None)?;
    if let Err(err) = sync_main_index_to_head(repo_root) {
        tracing::warn!(
            deve_commit_id = %record.deve_commit_id,
            git_commit_id = %git_commit,
            error = %err,
            "Git mirror snapshot bootstrap committed but failed to refresh main Git index"
        );
    }
    Ok(git_commit)
}

fn preflight_snapshot_bootstrap(
    db: &Database,
    repo_root: &Path,
    commit: &CommitInfo,
) -> GitSnapshotBootstrapResult<()> {
    ensure_git_worktree(repo_root)?;
    ensure_notegit_is_not_tracked(repo_root)?;
    ensure_source_control_clean(db)?;
    if let Some(head) = git_cmd::current_head(repo_root)? {
        return Err(GitSnapshotBootstrapError::NonEmptyGitHistory { head });
    }
    let files = source_control::commit_diff::projection_files_at_commit(db, &commit.id)
        .map_err(map_projection_snapshot_inspect_error)?;
    ensure_git_changes_match_snapshot_paths(repo_root, files.into_iter().map(|file| file.path))?;
    Ok(())
}

fn map_projection_snapshot_inspect_error(
    err: source_control::CommitDiffError,
) -> GitSnapshotBootstrapError {
    if is_snapshot_storage_error(&err) {
        return GitSnapshotBootstrapError::ProjectionSnapshotInspectStorage {
            message: err.to_string(),
        };
    }
    GitSnapshotBootstrapError::ProjectionSnapshotInspect {
        message: err.to_string(),
    }
}

fn map_projection_snapshot_load_error(
    err: source_control::CommitDiffError,
) -> GitSnapshotBootstrapError {
    if is_snapshot_storage_error(&err) {
        return GitSnapshotBootstrapError::ProjectionSnapshotLoadStorage {
            message: err.to_string(),
        };
    }
    GitSnapshotBootstrapError::ProjectionSnapshotLoad {
        message: err.to_string(),
    }
}

fn is_snapshot_storage_error(err: &source_control::CommitDiffError) -> bool {
    matches!(
        err,
        source_control::CommitDiffError::CommitTable { .. }
            | source_control::CommitDiffError::CommitLoad { .. }
            | source_control::CommitDiffError::CommitNotFound { .. }
            | source_control::CommitDiffError::CommitDecode { .. }
            | source_control::CommitDiffError::LedgerRange { .. }
            | source_control::CommitDiffError::ContentLoad { .. }
    )
}
