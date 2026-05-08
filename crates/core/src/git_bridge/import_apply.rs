//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Explicit Git import apply. It writes only Source Control pending state;
//! ledger commit authority remains the normal Stage -> Commit workflow.

use super::error::{GitImportApplyError, GitImportApplyResult};
use super::import_plan::{GitImportPlan, GitImportPlanBlocker, GitImportPlanEntry, plan_import};
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::{ChangeStatus, conflict, pending_fs, staging};
use crate::utils::path::join_normalized;
use anyhow::Result;
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitImportApplyReport {
    pub plan: GitImportPlan,
    pub applied: usize,
    pub skipped: usize,
    pub blockers: Vec<GitImportPlanBlocker>,
}

pub fn apply_import(
    repo: &RepoManager,
    repo_name: &str,
    repo_root: &Path,
) -> Result<GitImportApplyReport> {
    let plan = plan_import(repo_root)?;
    let mut apply_blockers = Vec::new();
    let mut candidates = Vec::new();
    if plan.blockers.is_empty() {
        for entry in &plan.entries {
            match build_pending_entry(repo, repo_name, repo_root, entry) {
                Ok(candidate) => candidates.push(candidate),
                Err(reason) => {
                    let reason: String = reason.into();
                    apply_blockers.push(GitImportPlanBlocker {
                        path: entry.path.clone(),
                        reason,
                    });
                }
            }
        }
    }

    let mut report = GitImportApplyReport {
        plan,
        applied: 0,
        skipped: 0,
        blockers: apply_blockers,
    };
    if !report.plan.blockers.is_empty() || !report.blockers.is_empty() || candidates.is_empty() {
        return Ok(report);
    }

    let (applied, skipped, blockers) = repo.run_on_local_repo(repo_name, |db| {
        apply_pending_candidates(db, &candidates).map_err(anyhow::Error::from)
    })?;
    report.applied = applied;
    report.skipped = skipped;
    report.blockers = blockers;
    Ok(report)
}

fn build_pending_entry(
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

fn apply_pending_candidates(
    db: &Database,
    candidates: &[pending_fs::PendingFsEntry],
) -> GitImportApplyResult<(usize, usize, Vec<GitImportPlanBlocker>)> {
    let blockers = preflight_pending_apply(db, candidates)?;
    if !blockers.is_empty() {
        return Ok((0, 0, blockers));
    }
    let written = pending_fs::upsert_many(db, candidates).map_err(|err| {
        GitImportApplyError::PendingEntryWrite {
            message: err.to_string(),
        }
    })?;
    Ok((
        written,
        candidates.len().saturating_sub(written),
        Vec::new(),
    ))
}

fn preflight_pending_apply(
    db: &Database,
    candidates: &[pending_fs::PendingFsEntry],
) -> GitImportApplyResult<Vec<GitImportPlanBlocker>> {
    let mut blockers = Vec::new();
    let staged =
        staging::list_staged_entries(db).map_err(|err| GitImportApplyError::StagedInspect {
            message: err.to_string(),
        })?;
    if !staged.is_empty() {
        blockers.push(GitImportPlanBlocker {
            path: "-".to_string(),
            reason: format!(
                "Git import apply refuses to run with {} source-control staged change(s)",
                staged.len()
            ),
        });
    }

    let mut seen = BTreeSet::new();
    for entry in candidates {
        if !seen.insert(entry.path.clone()) {
            blockers.push(GitImportPlanBlocker {
                path: entry.path.clone(),
                reason: "Git import apply refuses duplicate pending target".to_string(),
            });
        }
        if let Some(existing) = pending_entry_at(db, &entry.path)?
            && !pending_fs::semantic_eq(&existing, entry)
        {
            blockers.push(GitImportPlanBlocker {
                path: entry.path.clone(),
                reason: "Git import apply refuses to overwrite existing pending entry".to_string(),
            });
        }
        if let Some(previous_path) = entry.renamed_from.as_deref()
            && previous_path != entry.path
            && pending_entry_at(db, previous_path)?.is_some()
        {
            blockers.push(GitImportPlanBlocker {
                path: previous_path.to_string(),
                reason: "Git import apply refuses existing pending entry at rename source"
                    .to_string(),
            });
        }
    }
    Ok(blockers)
}

fn pending_entry_at(
    db: &Database,
    path: &str,
) -> GitImportApplyResult<Option<pending_fs::PendingFsEntry>> {
    pending_fs::get(db, path).map_err(|err| GitImportApplyError::PendingEntryInspect {
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

#[cfg(test)]
#[path = "import_apply_test.rs"]
mod tests;
