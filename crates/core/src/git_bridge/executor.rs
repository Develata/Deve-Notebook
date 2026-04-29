//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Explicit Git mirror executor. It never acts as source-control authority.

use super::git_cmd;
use super::preflight::{
    ensure_git_changes_match_deve_commit, ensure_git_worktree, ensure_notegit_is_not_tracked,
    ensure_source_control_clean,
};
use super::replay::run_projection_replay;
use super::status::{GitMirrorState, inspect_repo_root};
use super::store::{
    GitMirrorCommitState, GitMirrorRecord, list_records, mark_committed, mark_out_of_sync,
};
use anyhow::Result;
use redb::Database;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GitMirrorRunOptions {
    pub retry_out_of_sync: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitMirrorRunReport {
    pub attempted: usize,
    pub committed: usize,
    pub out_of_sync: usize,
    pub skipped: usize,
    pub records: Vec<GitMirrorRecord>,
}

pub fn run_pending_mirror(
    db: &Database,
    repo_root: &Path,
    options: GitMirrorRunOptions,
) -> Result<GitMirrorRunReport> {
    let candidates = pending_candidates(db, options.retry_out_of_sync)?;
    if candidates.is_empty() {
        return Ok(GitMirrorRunReport::default());
    }

    let status = inspect_repo_root(repo_root)?;
    if status.state != GitMirrorState::Ready {
        let reason = status.reason.unwrap_or_else(|| {
            format!(
                "Git mirror is not ready: state={} git={}",
                status.state.as_str(),
                status.git_metadata_kind.as_str()
            )
        });
        return mark_all_out_of_sync(db, candidates, reason);
    }

    if candidates.len() > 1 {
        return run_projection_replay(db, repo_root, candidates);
    }

    run_one_candidate(db, repo_root, &candidates[0])
}

fn pending_candidates(db: &Database, retry_out_of_sync: bool) -> Result<Vec<GitMirrorRecord>> {
    Ok(list_records(db)?
        .into_iter()
        .filter(|record| {
            record.state == GitMirrorCommitState::Queued
                || (retry_out_of_sync && record.state == GitMirrorCommitState::OutOfSync)
        })
        .collect())
}

fn run_one_candidate(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> Result<GitMirrorRunReport> {
    let mut report = GitMirrorRunReport {
        attempted: 1,
        ..GitMirrorRunReport::default()
    };
    match commit_worktree(db, repo_root, record) {
        Ok(git_commit_id) => {
            let updated = mark_committed(db, &record.deve_commit_id, &git_commit_id)?;
            report.committed = 1;
            report.records.push(updated);
        }
        Err(err) => {
            let updated = mark_out_of_sync(db, &record.deve_commit_id, err)?;
            report.out_of_sync = 1;
            report.records.push(updated);
        }
    }
    Ok(report)
}

fn mark_all_out_of_sync(
    db: &Database,
    records: Vec<GitMirrorRecord>,
    reason: String,
) -> Result<GitMirrorRunReport> {
    let mut report = GitMirrorRunReport {
        attempted: records.len(),
        ..GitMirrorRunReport::default()
    };
    for record in records {
        let updated = mark_out_of_sync(db, &record.deve_commit_id, reason.clone())?;
        report.out_of_sync += 1;
        report.records.push(updated);
    }
    Ok(report)
}

fn commit_worktree(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> std::result::Result<String, String> {
    preflight_mirror_commit(db, repo_root, record)?;
    git_cmd::run(repo_root, &["add", "-A"])?;
    if !git_cmd::has_staged_changes(repo_root)? {
        if let Some(git_commit_id) = matching_head_commit(repo_root, record)? {
            return Ok(git_commit_id);
        }
        return Err("git mirror has no staged changes for queued Deve commit".to_string());
    }
    git_cmd::run(
        repo_root,
        &["commit", "--no-gpg-sign", "-m", &commit_message(record)],
    )?;
    Ok(git_cmd::run(repo_root, &["rev-parse", "HEAD"])?
        .trim()
        .to_string())
}

pub(super) fn commit_message(record: &GitMirrorRecord) -> String {
    format!(
        "Mirror Deve commit {}\n\nDeve-Commit-Id: {}\nDeve-Ledger-Seq: {}\nDeve-Repo-Id: {}",
        record.deve_commit_id, record.deve_commit_id, record.ledger_seq, record.repo_id
    )
}

fn preflight_mirror_commit(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> std::result::Result<(), String> {
    ensure_git_worktree(repo_root)?;
    ensure_notegit_is_not_tracked(repo_root)?;
    ensure_source_control_clean(db)?;
    ensure_git_changes_match_deve_commit(db, repo_root, record)?;
    Ok(())
}

fn matching_head_commit(
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> std::result::Result<Option<String>, String> {
    let body = match git_cmd::run(repo_root, &["log", "-1", "--pretty=%B"]) {
        Ok(body) => body,
        Err(_) => return Ok(None),
    };
    if !commit_body_matches_record(&body, record) {
        return Ok(None);
    }
    Ok(Some(
        git_cmd::run(repo_root, &["rev-parse", "HEAD"])?
            .trim()
            .to_string(),
    ))
}

fn commit_body_matches_record(body: &str, record: &GitMirrorRecord) -> bool {
    let expected_commit = format!("Deve-Commit-Id: {}", record.deve_commit_id);
    let expected_seq = format!("Deve-Ledger-Seq: {}", record.ledger_seq);
    let expected_repo = format!("Deve-Repo-Id: {}", record.repo_id);
    let has_commit = body.lines().any(|line| line.trim() == expected_commit);
    let has_seq = body.lines().any(|line| line.trim() == expected_seq);
    let has_repo = body.lines().any(|line| line.trim() == expected_repo);
    has_commit && has_seq && has_repo
}

#[cfg(test)]
#[path = "executor_test.rs"]
mod executor_test;
