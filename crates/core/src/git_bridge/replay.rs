//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Projection replay for accumulated Git mirror records.

use super::executor::{GitMirrorRunReport, commit_message};
use super::git_cmd;
use super::preflight::{
    ensure_git_changes_match_deve_commits, ensure_git_worktree, ensure_notegit_is_not_tracked,
    ensure_source_control_clean, load_deve_commit,
};
use super::replay_git::{
    add_gitignore_to_index, apply_diff_to_index, commit_tree, ensure_git_commit_exists,
    read_parent_tree, sync_main_index_to_head, update_head,
};
use super::store::{
    GitMirrorCommitState, GitMirrorRecord, get_record, mark_committed, mark_out_of_sync,
};
use crate::source_control::{self, CommitInfo};
use anyhow::Result;
use redb::Database;
use std::path::Path;
struct ReplayItem {
    record: GitMirrorRecord,
    commit: CommitInfo,
}
pub(super) fn run_projection_replay(
    db: &Database,
    repo_root: &Path,
    records: Vec<GitMirrorRecord>,
) -> Result<GitMirrorRunReport> {
    let mut report = GitMirrorRunReport {
        attempted: records.len(),
        ..GitMirrorRunReport::default()
    };

    let items = match prepare_replay(db, repo_root, records) {
        Ok(items) => items,
        Err((records, reason)) => {
            return mark_remaining_out_of_sync(db, &mut report, records, reason);
        }
    };

    let temp_dir = match tempfile::tempdir() {
        Ok(temp_dir) => temp_dir,
        Err(err) => {
            let records = items.into_iter().map(|item| item.record).collect();
            return mark_remaining_out_of_sync(
                db,
                &mut report,
                records,
                format!("failed to create temporary Git mirror index: {err}"),
            );
        }
    };
    let index_path = temp_dir.path().join("mirror.index");
    let mut parent_git = match initial_git_parent(db, repo_root, &items[0].commit) {
        Ok(parent) => parent,
        Err(reason) => {
            let records = items.into_iter().map(|item| item.record).collect();
            return mark_remaining_out_of_sync(db, &mut report, records, reason);
        }
    };

    let mut remaining = items;
    while !remaining.is_empty() {
        let item = remaining.remove(0);
        match create_git_commit_from_projection(
            db,
            repo_root,
            &index_path,
            parent_git.as_deref(),
            &item,
        ) {
            Ok(git_commit_id) => {
                let updated = mark_committed(db, &item.record.deve_commit_id, &git_commit_id)?;
                report.committed += 1;
                report.records.push(updated);
                parent_git = Some(git_commit_id);
            }
            Err(reason) => {
                let mut records = Vec::with_capacity(remaining.len() + 1);
                records.push(item.record);
                records.extend(remaining.into_iter().map(|item| item.record));
                return mark_remaining_out_of_sync(
                    db,
                    &mut report,
                    records,
                    format!("Git mirror projection replay failed: {reason}"),
                );
            }
        }
    }

    Ok(report)
}
fn prepare_replay(
    db: &Database,
    repo_root: &Path,
    records: Vec<GitMirrorRecord>,
) -> std::result::Result<Vec<ReplayItem>, (Vec<GitMirrorRecord>, String)> {
    if let Err(reason) = preflight_replay(db, repo_root, &records) {
        return Err((records, reason));
    }
    match load_replay_items(db, records) {
        Ok(items) => Ok(items),
        Err((records, reason)) => Err((records, reason)),
    }
}

fn preflight_replay(
    db: &Database,
    repo_root: &Path,
    records: &[GitMirrorRecord],
) -> std::result::Result<(), String> {
    ensure_git_worktree(repo_root)?;
    ensure_notegit_is_not_tracked(repo_root)?;
    ensure_source_control_clean(db)?;
    ensure_git_changes_match_deve_commits(db, repo_root, records)
}

fn load_replay_items(
    db: &Database,
    records: Vec<GitMirrorRecord>,
) -> std::result::Result<Vec<ReplayItem>, (Vec<GitMirrorRecord>, String)> {
    let mut items = Vec::with_capacity(records.len());
    for record in &records {
        let commit = match load_deve_commit(db, &record.deve_commit_id) {
            Ok(commit) => commit,
            Err(reason) => return Err((records.clone(), reason)),
        };
        if commit.ledger_seq != record.ledger_seq {
            return Err((
                records.clone(),
                format!(
                    "Git mirror record ledger_seq {} does not match Deve commit ledger_seq {}",
                    record.ledger_seq, commit.ledger_seq
                ),
            ));
        }
        items.push(ReplayItem {
            record: record.clone(),
            commit,
        });
    }
    items.sort_by(|left, right| {
        left.commit
            .ledger_seq
            .cmp(&right.commit.ledger_seq)
            .then(left.commit.id.cmp(&right.commit.id))
    });
    validate_contiguous_chain(&items).map_err(|reason| {
        let records = items.iter().map(|item| item.record.clone()).collect();
        (records, reason)
    })?;
    Ok(items)
}

fn validate_contiguous_chain(items: &[ReplayItem]) -> std::result::Result<(), String> {
    for pair in items.windows(2) {
        let previous = &pair[0].commit.id;
        if pair[1].commit.parent_id.as_deref() != Some(previous.as_str()) {
            return Err(format!(
                "queued Git mirror records are not a contiguous Deve commit chain: {} parent is {:?}, expected {}",
                pair[1].commit.id, pair[1].commit.parent_id, previous
            ));
        }
    }
    Ok(())
}

fn initial_git_parent(
    db: &Database,
    repo_root: &Path,
    first_commit: &CommitInfo,
) -> std::result::Result<Option<String>, String> {
    let head = git_cmd::current_head(repo_root)?;
    let Some(parent_id) = first_commit.parent_id.as_deref() else {
        return Ok(head);
    };
    let parent_record = get_record(db, parent_id)
        .map_err(|err| format!("failed to read parent Git mirror record {parent_id}: {err}"))?
        .ok_or_else(|| {
            format!("first queued Git mirror commit parent {parent_id} is not mirrored")
        })?;
    if parent_record.state != GitMirrorCommitState::Committed {
        return Err(format!(
            "first queued Git mirror commit parent {parent_id} is {}",
            parent_record.state.as_str()
        ));
    }
    let git_parent = parent_record.git_commit_id.ok_or_else(|| {
        format!("committed parent Git mirror record {parent_id} has no git_commit_id")
    })?;
    ensure_git_commit_exists(repo_root, &git_parent)?;
    if head.as_deref() != Some(git_parent.as_str()) {
        return Err(format!(
            "Git HEAD does not match mirrored parent {parent_id}: head={:?} expected={}",
            head, git_parent
        ));
    }
    Ok(Some(git_parent))
}

fn create_git_commit_from_projection(
    db: &Database,
    repo_root: &Path,
    index_path: &Path,
    parent_git: Option<&str>,
    item: &ReplayItem,
) -> std::result::Result<String, String> {
    read_parent_tree(repo_root, index_path, parent_git)?;
    let diffs = source_control::commit_diff::compare_commits(
        db,
        item.commit.parent_id.as_deref(),
        &item.commit.id,
    )
    .map_err(|err| {
        format!(
            "failed to compute projection diff for {}: {err}",
            item.commit.id
        )
    })?;
    if diffs.is_empty() {
        return Err(format!(
            "Deve commit {} has no projection diff to mirror",
            item.commit.id
        ));
    }
    for diff in &diffs {
        apply_diff_to_index(repo_root, index_path, diff)?;
    }
    add_gitignore_to_index(repo_root, index_path)?;
    let tree = git_cmd::run_env(
        repo_root,
        &["write-tree"],
        &[("GIT_INDEX_FILE", index_path)],
    )?
    .trim()
    .to_string();
    let git_commit = commit_tree(repo_root, &tree, parent_git, &commit_message(&item.record))?;
    update_head(repo_root, &git_commit, parent_git)?;
    if let Err(err) = sync_main_index_to_head(repo_root) {
        tracing::warn!(
            deve_commit_id = %item.record.deve_commit_id,
            git_commit_id = %git_commit,
            error = %err,
            "Git mirror committed but failed to refresh main Git index"
        );
    }
    Ok(git_commit)
}

fn mark_remaining_out_of_sync(
    db: &Database,
    report: &mut GitMirrorRunReport,
    records: Vec<GitMirrorRecord>,
    reason: String,
) -> Result<GitMirrorRunReport> {
    for record in records {
        let updated = mark_out_of_sync(db, &record.deve_commit_id, reason.clone())?;
        report.out_of_sync += 1;
        report.records.push(updated);
    }
    Ok(report.clone())
}
