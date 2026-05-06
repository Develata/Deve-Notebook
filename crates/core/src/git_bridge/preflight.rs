//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Shared preflight checks and Deve commit lookup for Git mirror execution.

use super::git_cmd;
use super::store::GitMirrorRecord;
use crate::source_control::{self, CommitInfo};
use crate::utils::{notegit, path::to_forward_slash};
use redb::Database;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn ensure_git_worktree(repo_root: &Path) -> std::result::Result<(), String> {
    let inside = git_cmd::run(repo_root, &["rev-parse", "--is-inside-work-tree"])?;
    if inside.trim() == "true" {
        return Ok(());
    }
    Err(format!(
        "Git mirror is not a usable worktree: rev-parse returned {}",
        inside.trim()
    ))
}

pub(super) fn ensure_notegit_is_not_tracked(repo_root: &Path) -> std::result::Result<(), String> {
    let paths = git_cmd::run_z_paths(repo_root, &["ls-files", "-z", "--", notegit::NOTE_GIT_DIR])?;
    if paths.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Git mirror refuses to run because {} is already tracked by Git",
        notegit::NOTE_GIT_DIR
    ))
}

pub(super) fn ensure_source_control_clean(db: &Database) -> std::result::Result<(), String> {
    let pending = source_control::pending_fs::list_all(db)
        .map_err(|err| format!("failed to inspect pending source-control changes: {err}"))?;
    if !pending.is_empty() {
        return Err(format!(
            "Git mirror refuses to run with {} pending source-control change(s)",
            pending.len()
        ));
    }

    let staged = source_control::staging::list_staged_entries(db)
        .map_err(|err| format!("failed to inspect staged source-control changes: {err}"))?;
    if !staged.is_empty() {
        return Err(format!(
            "Git mirror refuses to run with {} staged source-control change(s)",
            staged.len()
        ));
    }
    Ok(())
}

pub(super) fn ensure_git_worktree_clean(repo_root: &Path) -> std::result::Result<(), String> {
    let changed = git_changed_paths(repo_root)?;
    if changed.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Git mirror refuses to push dirty Git worktree path(s): {}",
        changed.into_iter().collect::<Vec<_>>().join(", ")
    ))
}

pub(super) fn ensure_git_changes_match_deve_commit(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> std::result::Result<(), String> {
    let expected = expected_mirror_paths(db, record)?;
    ensure_git_changes_within(repo_root, expected, "queued Deve commit")
}

pub(super) fn ensure_git_changes_match_deve_commits(
    db: &Database,
    repo_root: &Path,
    records: &[GitMirrorRecord],
) -> std::result::Result<(), String> {
    let mut expected = BTreeSet::new();
    for record in records {
        expected.extend(expected_mirror_paths(db, record)?);
    }
    ensure_git_changes_within(repo_root, expected, "queued Deve commits")
}

pub(super) fn ensure_git_changes_match_snapshot_paths(
    repo_root: &Path,
    paths: impl IntoIterator<Item = String>,
) -> std::result::Result<(), String> {
    let mut expected = BTreeSet::new();
    expected.insert(".gitignore".to_string());
    expected.extend(paths.into_iter().map(|path| to_forward_slash(&path)));
    ensure_git_changes_within(repo_root, expected, "current Deve projection snapshot")
}

pub(super) fn expected_mirror_paths(
    db: &Database,
    record: &GitMirrorRecord,
) -> std::result::Result<BTreeSet<String>, String> {
    let commit = load_deve_commit(db, &record.deve_commit_id)?;
    if commit.ledger_seq != record.ledger_seq {
        return Err(format!(
            "Git mirror record ledger_seq {} does not match Deve commit ledger_seq {}",
            record.ledger_seq, commit.ledger_seq
        ));
    }

    let diffs =
        source_control::commit_diff::compare_commits(db, commit.parent_id.as_deref(), &commit.id)
            .map_err(|err| format!("failed to compute queued Deve commit diff: {err}"))?;
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

pub(super) fn load_deve_commit(
    db: &Database,
    commit_id: &str,
) -> std::result::Result<CommitInfo, String> {
    let read_txn = db
        .begin_read()
        .map_err(|err| format!("failed to read Deve commit table: {err}"))?;
    let table = read_txn
        .open_table(source_control::commits::COMMITS_TABLE)
        .map_err(|err| format!("failed to open Deve commit table: {err}"))?;
    let raw = table
        .get(commit_id)
        .map_err(|err| format!("failed to load Deve commit {commit_id}: {err}"))?
        .ok_or_else(|| {
            format!("queued Git mirror record references missing Deve commit {commit_id}")
        })?;
    serde_json::from_str(raw.value())
        .map_err(|err| format!("failed to decode Deve commit {commit_id}: {err}"))
}

fn ensure_git_changes_within(
    repo_root: &Path,
    expected: BTreeSet<String>,
    scope: &str,
) -> std::result::Result<(), String> {
    let changed = git_changed_paths(repo_root)?;
    let unexpected: Vec<_> = changed
        .into_iter()
        .filter(|path| notegit::is_internal_repo_path(path) || !expected.contains(path))
        .collect();
    if unexpected.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Git mirror refuses to include path(s) outside {scope}: {}",
        unexpected.join(", ")
    ))
}

fn git_changed_paths(repo_root: &Path) -> std::result::Result<BTreeSet<String>, String> {
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
