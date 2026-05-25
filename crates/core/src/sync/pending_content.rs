//! # Pending 内容同步辅助
//! plan_ref:
//!   - 03_storage#watcher-contract
//!   - 05_diff_logic#source-control-runtime
//!
//! Invariants:
//! - Watcher 只能依据当前 Ledger projection 与磁盘内容比较，决定清理或更新 pending。
//! - 比较结果只影响 pending side table，不得直接改写 Ledger。

use super::pending;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::ChangeStatus;
use crate::source_control::conflict;
use crate::source_control::pending_fs;
use anyhow::Result;
use std::sync::Arc;

pub(super) enum PendingSyncResult {
    Noop,
    Changed,
}

pub(super) fn sync_modified_pending(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    repo_path: &str,
    doc_id: DocId,
) -> Result<PendingSyncResult> {
    let file_path = repo.local_repo_workspace_path(repo_name, repo_path)?;
    let disk_content = std::fs::read_to_string(&file_path)?;
    let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)?;
    let current = repo.run_on_local_repo(repo_name, |db| pending_fs::get(db, repo_path))?;
    if equivalent_reconciled_content(&rebuilt.content, &disk_content) {
        return if current.is_some() {
            pending::clear(repo, repo_name, repo_path)?;
            Ok(PendingSyncResult::Changed)
        } else {
            Ok(PendingSyncResult::Noop)
        };
    }
    let next_hash = pending_fs::content_hash(&disk_content);
    let has_conflict = repo.run_on_local_repo(repo_name, |db| {
        conflict::check_conflict(db, doc_id, &next_hash)
    })?;
    let unchanged = matches!(
        current,
        Some(entry)
            if entry.change_type == ChangeStatus::Modified
                && entry.doc_id == Some(doc_id)
                && entry.renamed_from.is_none()
                && entry.content_hash == next_hash
                && entry.has_conflict == has_conflict
    );
    if unchanged {
        return Ok(PendingSyncResult::Noop);
    }
    pending::upsert(
        repo,
        repo_name,
        repo_path,
        ChangeStatus::Modified,
        Some(doc_id),
        None,
    )?;
    Ok(PendingSyncResult::Changed)
}

fn equivalent_reconciled_content(left: &str, right: &str) -> bool {
    left == right || left.replace("\r\n", "\n") == right.replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::equivalent_reconciled_content;

    #[test]
    fn reconciled_content_equivalence_ignores_crlf_only_drift() {
        assert!(equivalent_reconciled_content("a\nb\n", "a\r\nb\r\n"));
        assert!(!equivalent_reconciled_content("a\nb\n", "a\nchanged\n"));
    }
}
