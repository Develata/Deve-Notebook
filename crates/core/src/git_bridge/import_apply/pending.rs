//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 14_commands#cli-commands
//!

use super::{GitImportApplyError, GitImportApplyResult, GitImportPlanBlocker};
use crate::source_control::{pending_fs, staging};
use redb::Database;
use std::collections::BTreeSet;

pub(super) fn apply_pending_candidates(
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
