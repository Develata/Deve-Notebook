//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Projection replay for accumulated Git mirror records.

use super::error::{GitMirrorRunResult, GitProjectionReplayError, GitProjectionReplayResult};
use super::executor::{GitMirrorRunReport, commit_message};
use super::git_cmd;
use super::replay_git::{
    add_gitignore_to_index, apply_diff_to_index, commit_tree, read_parent_tree,
    sync_main_index_to_head, update_head,
};
use super::replay_plan::{ReplayItem, initial_git_parent, prepare_replay};
use super::store::{GitMirrorRecord, mark_committed, mark_out_of_sync};
use crate::source_control;
use redb::Database;
use std::path::Path;

pub(super) fn run_projection_replay(
    db: &Database,
    repo_root: &Path,
    records: Vec<GitMirrorRecord>,
) -> GitMirrorRunResult<GitMirrorRunReport> {
    let mut report = GitMirrorRunReport {
        attempted: records.len(),
        ..GitMirrorRunReport::default()
    };

    let items = match prepare_replay(db, repo_root, records) {
        Ok(items) => items,
        Err((records, reason)) => {
            return mark_remaining_out_of_sync(db, &mut report, records, reason.into());
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
            return mark_remaining_out_of_sync(db, &mut report, records, reason.into());
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

fn create_git_commit_from_projection(
    db: &Database,
    repo_root: &Path,
    index_path: &Path,
    parent_git: Option<&str>,
    item: &ReplayItem,
) -> GitProjectionReplayResult<String> {
    read_parent_tree(repo_root, index_path, parent_git)?;
    let diffs = source_control::commit_diff::compare_commits(
        db,
        item.commit.parent_id.as_deref(),
        &item.commit.id,
    )
    .map_err(|err| GitProjectionReplayError::ProjectionDiff {
        commit_id: item.commit.id.clone(),
        message: err.to_string(),
    })?;
    if diffs.is_empty() {
        return Err(GitProjectionReplayError::EmptyProjectionDiff {
            commit_id: item.commit.id.clone(),
        });
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
) -> GitMirrorRunResult<GitMirrorRunReport> {
    for record in records {
        let updated = mark_out_of_sync(db, &record.deve_commit_id, reason.clone())?;
        report.out_of_sync += 1;
        report.records.push(updated);
    }
    Ok(report.clone())
}
