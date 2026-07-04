//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Terminal Git mirror commit builder for queued NoteGit records.

use super::error::{GitMirrorCommitError, GitMirrorCommitResult, GitPreflightError};
use super::executor::commit_message;
use super::git_cmd;
use super::preflight::{
    ensure_git_changes_match_deve_commits, ensure_git_worktree, ensure_notegit_is_not_tracked,
    ensure_source_control_clean, map_commit_diff_error,
};
use super::replay_git::{
    add_blob_to_index, add_gitignore_to_index, commit_tree, read_parent_tree,
    sync_main_index_to_head, update_head,
};
use super::store::GitMirrorRecord;
use crate::source_control::{self, CommitFileDiff};
use crate::utils::{notegit, path::to_forward_slash};
use redb::Database;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) fn commit_terminal_worktree(
    db: &Database,
    repo_root: &Path,
    records: &[GitMirrorRecord],
) -> GitMirrorCommitResult<String> {
    preflight_terminal_mirror_commit(db, repo_root, records)?;
    let latest = latest_record(records).ok_or(GitMirrorCommitError::NoStagedChanges)?;
    let files = source_control::commit_diff::projection_files_at_commit(db, &latest.deve_commit_id)
        .map_err(map_commit_diff_error)?;
    ensure_workspace_matches_terminal_projection(repo_root, &files)?;

    let temp_dir = tempfile::tempdir().map_err(|err| GitMirrorCommitError::TempIndex {
        message: err.to_string(),
    })?;
    let index_path = temp_dir.path().join("mirror-terminal.index");
    read_parent_tree(repo_root, &index_path, None)?;
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
    let parent = git_cmd::current_head(repo_root)?;
    let git_commit = commit_tree(repo_root, &tree, parent.as_deref(), &commit_message(latest))?;
    update_head(repo_root, &git_commit, parent.as_deref())?;
    if let Err(err) = sync_main_index_to_head(repo_root) {
        tracing::warn!(
            deve_commit_id = %latest.deve_commit_id,
            git_commit_id = %git_commit,
            error = %err,
            "Git mirror terminal commit succeeded but failed to refresh main Git index"
        );
    }
    Ok(git_commit)
}

fn preflight_terminal_mirror_commit(
    db: &Database,
    repo_root: &Path,
    records: &[GitMirrorRecord],
) -> GitMirrorCommitResult<()> {
    ensure_git_worktree(repo_root)?;
    ensure_notegit_is_not_tracked(repo_root)?;
    ensure_source_control_clean(db)?;
    ensure_git_changes_match_deve_commits(db, repo_root, records)?;
    Ok(())
}

fn latest_record(records: &[GitMirrorRecord]) -> Option<&GitMirrorRecord> {
    records.iter().max_by(|left, right| {
        left.ledger_seq
            .cmp(&right.ledger_seq)
            .then_with(|| left.deve_commit_id.cmp(&right.deve_commit_id))
    })
}

fn ensure_workspace_matches_terminal_projection(
    repo_root: &Path,
    files: &[CommitFileDiff],
) -> GitMirrorCommitResult<()> {
    let mut expected = BTreeSet::from([".gitignore".to_string()]);
    for file in files {
        let path = to_forward_slash(&file.path);
        expected.insert(path.clone());
        let abs = safe_projection_workspace_path(repo_root, &path)?;
        let current = std::fs::read_to_string(&abs).map_err(|err| {
            GitMirrorCommitError::GitPreflight(GitPreflightError::ProjectionContentMismatch {
                path: path.clone(),
                reason: format!("failed to read workspace file: {err}"),
            })
        })?;
        if current != file.new_content {
            return Err(GitMirrorCommitError::GitPreflight(
                GitPreflightError::ProjectionContentMismatch {
                    path,
                    reason: "workspace content differs from terminal NoteGit projection"
                        .to_string(),
                },
            ));
        }
    }

    ensure_no_unexpected_workspace_paths(repo_root, &expected)
}

fn ensure_no_unexpected_workspace_paths(
    repo_root: &Path,
    expected: &BTreeSet<String>,
) -> GitMirrorCommitResult<()> {
    let mut unexpected = BTreeSet::new();
    for args in [
        &["ls-files", "-z"][..],
        &["ls-files", "-o", "--exclude-standard", "-z"][..],
    ] {
        for path in git_cmd::run_z_paths(repo_root, args)? {
            let path = to_forward_slash(&path);
            if expected.contains(&path) || notegit::is_internal_repo_path(&path) {
                continue;
            }
            let abs = safe_projection_workspace_path(repo_root, &path)?;
            if abs.exists() {
                unexpected.insert(path);
            }
        }
    }
    if unexpected.is_empty() {
        return Ok(());
    }
    Err(GitMirrorCommitError::GitPreflight(
        GitPreflightError::ProjectionScope {
            scope: "current Deve projection snapshot".to_string(),
            paths: unexpected.into_iter().collect::<Vec<_>>().join(", "),
        },
    ))
}

fn safe_projection_workspace_path(repo_root: &Path, path: &str) -> GitMirrorCommitResult<PathBuf> {
    let path = to_forward_slash(path);
    if path.is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || notegit::is_internal_repo_path(&path)
    {
        return Err(GitMirrorCommitError::GitPreflight(
            GitPreflightError::ProjectionScope {
                scope: "current Deve projection snapshot".to_string(),
                paths: path,
            },
        ));
    }
    Ok(repo_root.join(path))
}
