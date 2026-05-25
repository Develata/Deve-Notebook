//! plan_ref:
//!   - 03_storage#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Preflight, commit loading, and chain validation for Git projection replay.

use super::error::{GitReplayPlanError, GitReplayPlanResult};
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
) -> std::result::Result<Vec<ReplayItem>, (Vec<GitMirrorRecord>, GitReplayPlanError)> {
    if let Err(reason) = preflight_replay(db, repo_root, &records) {
        return Err((records, reason));
    }
    load_replay_items(db, records)
}

fn preflight_replay(
    db: &Database,
    repo_root: &Path,
    records: &[GitMirrorRecord],
) -> GitReplayPlanResult<()> {
    ensure_git_worktree(repo_root)?;
    ensure_notegit_is_not_tracked(repo_root)?;
    ensure_source_control_clean(db)?;
    ensure_git_changes_match_deve_commits(db, repo_root, records)?;
    Ok(())
}

fn load_replay_items(
    db: &Database,
    records: Vec<GitMirrorRecord>,
) -> std::result::Result<Vec<ReplayItem>, (Vec<GitMirrorRecord>, GitReplayPlanError)> {
    let mut items = Vec::with_capacity(records.len());
    for record in &records {
        let commit = match load_deve_commit(db, &record.deve_commit_id) {
            Ok(commit) => commit,
            Err(reason) => return Err((records.clone(), reason.into())),
        };
        if commit.ledger_seq != record.ledger_seq {
            return Err((
                records.clone(),
                GitReplayPlanError::MirrorRecordSeqMismatch {
                    record_seq: record.ledger_seq,
                    commit_seq: commit.ledger_seq,
                },
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

fn validate_contiguous_chain(items: &[ReplayItem]) -> GitReplayPlanResult<()> {
    for pair in items.windows(2) {
        let previous = &pair[0].commit.id;
        if pair[1].commit.parent_id.as_deref() != Some(previous.as_str()) {
            return Err(GitReplayPlanError::NonContiguousCommitChain {
                commit_id: pair[1].commit.id.clone(),
                parent: pair[1].commit.parent_id.clone(),
                expected: previous.clone(),
            });
        }
    }
    Ok(())
}

pub(super) fn initial_git_parent(
    db: &Database,
    repo_root: &Path,
    first_commit: &CommitInfo,
) -> GitReplayPlanResult<Option<String>> {
    let head = git_cmd::current_head(repo_root)?;
    let Some(parent_id) = first_commit.parent_id.as_deref() else {
        return Ok(head);
    };
    let parent_record = get_record(db, parent_id)
        .map_err(|err| GitReplayPlanError::ParentRecordRead {
            parent_id: parent_id.to_string(),
            message: err.to_string(),
        })?
        .ok_or_else(|| GitReplayPlanError::ParentNotMirrored {
            parent_id: parent_id.to_string(),
        })?;
    if parent_record.state != GitMirrorCommitState::Committed {
        return Err(GitReplayPlanError::ParentStateNotCommitted {
            parent_id: parent_id.to_string(),
            state: parent_record.state.as_str().to_string(),
        });
    }
    let git_parent =
        parent_record
            .git_commit_id
            .ok_or_else(|| GitReplayPlanError::ParentMissingGitCommit {
                parent_id: parent_id.to_string(),
            })?;
    ensure_git_commit_exists(repo_root, &git_parent)?;
    if head.as_deref() != Some(git_parent.as_str()) {
        return Err(GitReplayPlanError::HeadMismatch {
            parent_id: parent_id.to_string(),
            head,
            expected: git_parent,
        });
    }
    Ok(Some(git_parent))
}
