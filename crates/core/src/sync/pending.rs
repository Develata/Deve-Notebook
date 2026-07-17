//! # Watcher Pending 辅助
//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 05_diff_logic#source-control-runtime
//!
//! Invariants:
//! - Watcher 只能读写 pending side table。
//! - `WatcherRefresh` 只反映当前 pending 视图，不推导权威状态。

use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use crate::sync::watcher::{WatcherRefresh, WatcherRefreshKind};
use anyhow::{Result, anyhow};
use std::path::Path;
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
    if status == ChangeStatus::Deleted {
        return upsert_with_hash(
            repo,
            repo_name,
            path,
            status,
            doc_id_hint,
            renamed_from,
            String::new(),
        );
    }
    let file_path = repo.local_repo_workspace_path(repo_name, path)?;
    upsert_from_disk_path(
        repo,
        repo_name,
        path,
        status,
        doc_id_hint,
        renamed_from,
        &file_path,
    )
}

pub(super) fn upsert_from_disk_path(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    path: &str,
    status: ChangeStatus,
    doc_id_hint: Option<DocId>,
    renamed_from: Option<&str>,
    disk_path: &Path,
) -> Result<bool> {
    if status == ChangeStatus::Deleted {
        return Err(anyhow!(
            "deleted pending entries must not depend on a disk path"
        ));
    }
    let content = std::fs::read_to_string(disk_path)?;
    upsert_with_content(
        repo,
        repo_name,
        path,
        status,
        doc_id_hint,
        renamed_from,
        &content,
    )
}

pub(super) fn upsert_with_content(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    path: &str,
    status: ChangeStatus,
    doc_id_hint: Option<DocId>,
    renamed_from: Option<&str>,
    content: &str,
) -> Result<bool> {
    if status == ChangeStatus::Deleted {
        return Err(anyhow!(
            "deleted pending entries must not carry disk content"
        ));
    }
    upsert_with_hash(
        repo,
        repo_name,
        path,
        status,
        doc_id_hint,
        renamed_from,
        pending_fs::content_hash(content),
    )
}

fn upsert_with_hash(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    path: &str,
    status: ChangeStatus,
    doc_id_hint: Option<DocId>,
    renamed_from: Option<&str>,
    hash: String,
) -> Result<bool> {
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

pub(super) fn refresh(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    repo_id: crate::models::RepoId,
    path: &str,
    kind: WatcherRefreshKind,
) -> Result<WatcherRefresh> {
    let has_conflict = repo.run_on_local_repo(repo_name, |db| {
        Ok(pending_fs::get(db, path)?
            .map(|entry| entry.has_conflict)
            .unwrap_or(false))
    })?;
    Ok(WatcherRefresh::new(repo_id, path, kind, has_conflict))
}

pub(super) fn dir_changed_refresh(repo_id: crate::models::RepoId, path: &str) -> WatcherRefresh {
    WatcherRefresh::new(repo_id, path, WatcherRefreshKind::DirectoryChanged, false)
}

#[cfg(test)]
mod tests;
