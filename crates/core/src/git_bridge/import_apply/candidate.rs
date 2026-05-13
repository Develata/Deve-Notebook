//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!

use super::{GitImportApplyError, GitImportApplyResult, GitImportPlanEntry};
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::{ChangeStatus, conflict, pending_fs};
use crate::utils::path::join_normalized;
use std::path::Path;

pub(super) fn build_pending_entry(
    repo: &RepoManager,
    repo_name: &str,
    repo_root: &Path,
    entry: &GitImportPlanEntry,
) -> GitImportApplyResult<pending_fs::PendingFsEntry> {
    let content_hash = content_hash_for_entry(repo_root, entry)?;
    let doc_id = resolve_import_doc_id(repo, repo_name, entry)?;
    let has_conflict = match doc_id {
        Some(doc_id) => repo
            .run_on_local_repo(repo_name, |db| {
                conflict::check_conflict(db, doc_id, &content_hash)
            })
            .map_err(|err| GitImportApplyError::ConflictCheck {
                path: entry.path.clone(),
                message: err.to_string(),
            })?,
        None => false,
    };
    Ok(pending_fs::PendingFsEntry {
        path: entry.path.clone(),
        renamed_from: entry.previous_path.clone(),
        doc_id,
        change_type: entry.status,
        content_hash,
        detected_at: chrono::Utc::now().timestamp_millis(),
        has_conflict,
    })
}

fn content_hash_for_entry(
    repo_root: &Path,
    entry: &GitImportPlanEntry,
) -> GitImportApplyResult<String> {
    if entry.status == ChangeStatus::Deleted {
        return Ok(String::new());
    }
    let path = join_normalized(repo_root, &entry.path);
    let content = std::fs::read_to_string(&path).map_err(|err| {
        GitImportApplyError::ReadImportedWorktreeFile {
            path: entry.path.clone(),
            message: err.to_string(),
        }
    })?;
    Ok(pending_fs::content_hash(&content))
}

fn resolve_import_doc_id(
    repo: &RepoManager,
    repo_name: &str,
    entry: &GitImportPlanEntry,
) -> GitImportApplyResult<Option<DocId>> {
    match entry.status {
        ChangeStatus::Added => {
            if tracked_doc_id(repo, repo_name, &entry.path)?.is_some() {
                return Err(GitImportApplyError::AddedPathAlreadyTracked {
                    path: entry.path.clone(),
                });
            }
            Ok(None)
        }
        ChangeStatus::Modified | ChangeStatus::Deleted => {
            tracked_doc_id(repo, repo_name, &entry.path)?
                .ok_or_else(|| GitImportApplyError::MissingTrackedDoc {
                    status: change_status_label(entry.status),
                    path: entry.path.clone(),
                })
                .map(Some)
        }
        ChangeStatus::Renamed => resolve_rename_doc_id(repo, repo_name, entry),
    }
}

fn resolve_rename_doc_id(
    repo: &RepoManager,
    repo_name: &str,
    entry: &GitImportPlanEntry,
) -> GitImportApplyResult<Option<DocId>> {
    let previous_path = entry.previous_path.as_deref().ok_or_else(|| {
        GitImportApplyError::RenameMissingPreviousPath {
            path: entry.path.clone(),
        }
    })?;
    let doc_id = tracked_doc_id(repo, repo_name, previous_path)?.ok_or_else(|| {
        GitImportApplyError::RenameMissingTrackedDoc {
            previous_path: previous_path.to_string(),
        }
    })?;
    if let Some(current_doc) = tracked_doc_id(repo, repo_name, &entry.path)?
        && current_doc != doc_id
    {
        return Err(GitImportApplyError::RenameTargetAlreadyTracked {
            path: entry.path.clone(),
        });
    }
    Ok(Some(doc_id))
}

fn tracked_doc_id(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
) -> GitImportApplyResult<Option<DocId>> {
    repo.get_tracked_docid_in_local_repo(repo_name, path)
        .map_err(|err| GitImportApplyError::TrackedPathInspect {
            path: path.to_string(),
            message: err.to_string(),
        })
}

fn change_status_label(status: ChangeStatus) -> &'static str {
    match status {
        ChangeStatus::Added => "added",
        ChangeStatus::Modified => "modified",
        ChangeStatus::Deleted => "deleted",
        ChangeStatus::Renamed => "renamed",
    }
}
