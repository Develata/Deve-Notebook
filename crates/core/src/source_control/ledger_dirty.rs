//! Confirmed ledger dirty derivation.
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 05_diff_logic#authority-diff-core
//!   - 03_storage/authority#facts-partition
//!
//! This module derives user-visible ledger changes from the latest
//! source-control commit anchor to the current ledger head. It must remain a
//! pure read projection: no pending overlay, no pending_fs_ops, no side table.

use crate::protocol::ScPathTarget;
use crate::source_control::{
    ChangeDomain, ChangeEntry, CommitFileDiff, commit_diff, commits, diff,
};
use anyhow::{Result, anyhow};
use redb::Database;

pub fn latest_commit_seq(db: &Database) -> Result<u64> {
    let Some(commit_id) = commits::get_latest_id(db)? else {
        return Ok(0);
    };
    Ok(commits::get(db, &commit_id)?.ledger_seq)
}

pub fn ledger_head_seq(db: &Database) -> Result<u64> {
    crate::ledger::range::get_max_seq(db)
}

pub fn list_confirmed(db: &Database) -> Result<Vec<ChangeEntry>> {
    let base_seq = latest_commit_seq(db)?;
    let target_seq = ledger_head_seq(db)?;
    list_confirmed_between(db, base_seq, target_seq)
}

pub fn list_confirmed_between(
    db: &Database,
    base_seq: u64,
    target_seq: u64,
) -> Result<Vec<ChangeEntry>> {
    if target_seq <= base_seq {
        return Ok(Vec::new());
    }
    let diffs = commit_diff::compare_seq_range(db, base_seq, target_seq)?;
    Ok(diffs
        .into_iter()
        .map(|file| confirmed_entry(file, base_seq, target_seq))
        .collect())
}

pub fn has_confirmed_dirty(db: &Database) -> Result<bool> {
    Ok(!list_confirmed(db)?.is_empty())
}

pub fn diff_confirmed_target(db: &Database, target: &ScPathTarget) -> Result<String> {
    let base_seq = latest_commit_seq(db)?;
    let target_seq = ledger_head_seq(db)?;
    if target_seq <= base_seq {
        anyhow::bail!("No confirmed ledger changes");
    }
    let diffs = commit_diff::compare_seq_range(db, base_seq, target_seq)?;
    let file = diffs
        .into_iter()
        .find(|file| matches_target(file, target))
        .ok_or_else(|| anyhow!("Path is not in confirmed ledger changes: {}", target.path))?;
    Ok(diff::unified_diff(
        &file.old_content,
        &file.new_content,
        &file.path,
    ))
}

fn confirmed_entry(file: CommitFileDiff, base_seq: u64, target_seq: u64) -> ChangeEntry {
    ChangeEntry {
        path: file.path,
        renamed_from: file.previous_path,
        doc_id: file.doc_id,
        status: file.status,
        has_conflict: false,
        domain: ChangeDomain::ConfirmedLedger,
        base_seq: Some(base_seq),
        target_seq: Some(target_seq),
    }
}

fn matches_target(file: &CommitFileDiff, target: &ScPathTarget) -> bool {
    if let Some(doc_id) = target.doc_id {
        return file.doc_id == Some(doc_id);
    }
    file.path == target.path || file.previous_path.as_deref() == Some(target.path.as_str())
}
