//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Preflight, commit loading, and chain validation for Git projection replay.

use super::git_cmd;
use super::preflight::{
    ensure_git_changes_match_deve_commits, ensure_git_worktree, ensure_notegit_is_not_tracked,
    ensure_source_control_clean, load_deve_commit,
};
use super::replay_git::ensure_git_commit_exists;
use super::store::{GitMirrorCommitState, GitMirrorRecord, get_record};
use crate::source_control::CommitInfo;
use redb::Database;
use std::path::Path;

pub(super) struct ReplayItem {
    pub(super) record: GitMirrorRecord,
    pub(super) commit: CommitInfo,
}

pub(super) fn prepare_replay(
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
    ensure_git_changes_match_deve_commits(db, repo_root, records)?;
    Ok(())
}

fn load_replay_items(
    db: &Database,
    records: Vec<GitMirrorRecord>,
) -> std::result::Result<Vec<ReplayItem>, (Vec<GitMirrorRecord>, String)> {
    let mut items = Vec::with_capacity(records.len());
    for record in &records {
        let commit = match load_deve_commit(db, &record.deve_commit_id) {
            Ok(commit) => commit,
            Err(reason) => return Err((records.clone(), reason.into())),
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

pub(super) fn initial_git_parent(
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
