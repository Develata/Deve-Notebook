//! # Watcher Pending 辅助
//! plan_ref:
//!   - 04_storage#watcher-contract
//!   - 07_diff_logic#source-control-runtime
//!
//! Invariants:
//! - Watcher 只能读写 pending side table。
//! - `FsChangeDetected` 只反映当前 pending 视图，不推导权威状态。

use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::protocol::ServerMessage;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use anyhow::Result;
use std::sync::Arc;

pub(super) fn clear(repo: &Arc<RepoManager>, repo_name: &str, path: &str) -> Result<()> {
    repo.run_on_local_repo(repo_name, |db| pending_fs::remove(db, path))
}

pub(super) fn has_pending_added(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    path: &str,
) -> Result<bool> {
    repo.run_on_local_repo(repo_name, |db| {
        Ok(matches!(
            pending_fs::get(db, path)?,
            Some(entry) if entry.change_type == ChangeStatus::Added
        ))
    })
}

pub(super) fn upsert(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    path: &str,
    status: ChangeStatus,
    doc_id_hint: Option<DocId>,
    renamed_from: Option<&str>,
) -> Result<bool> {
    let hash = if status == ChangeStatus::Deleted {
        String::new()
    } else {
        let file_path = repo.local_repo_workspace_path(repo_name, path)?;
        let content = std::fs::read_to_string(&file_path)?;
        pending_fs::content_hash(&content)
    };
    let doc_id = match doc_id_hint {
        Some(doc_id) => Some(doc_id),
        None => repo.get_tracked_docid_in_local_repo(repo_name, path)?,
    };
    let has_conflict = match doc_id {
        Some(doc_id) => repo.run_on_local_repo(repo_name, |db| {
            crate::source_control::conflict::check_conflict(db, doc_id, &hash)
        })?,
        None => false,
    };
    let entry = PendingFsEntry {
        path: path.to_string(),
        renamed_from: renamed_from.map(str::to_string),
        doc_id,
        change_type: status,
        content_hash: hash,
        detected_at: chrono::Utc::now().timestamp_millis(),
        has_conflict,
    };
    repo.run_on_local_repo(repo_name, |db| {
        Ok(pending_fs::upsert_many(db, std::slice::from_ref(&entry))? > 0)
    })
}

pub(super) fn message(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    repo_id: crate::models::RepoId,
    path: &str,
    change_type: &str,
) -> Result<ServerMessage> {
    let has_conflict = repo.run_on_local_repo(repo_name, |db| {
        Ok(pending_fs::get(db, path)?
            .map(|entry| entry.has_conflict)
            .unwrap_or(false))
    })?;
    Ok(ServerMessage::FsChangeDetected {
        repo_id: Some(repo_id),
        branch: None,
        scope_nonce: None,
        path: path.to_string(),
        change_type: change_type.to_string(),
        has_conflict,
    })
}

#[cfg(test)]
#[path = "pending_test.rs"]
mod tests;
