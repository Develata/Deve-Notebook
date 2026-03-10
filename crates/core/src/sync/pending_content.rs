//! # Pending 内容同步辅助
//!
//! Invariants:
//! - Watcher 只能依据当前 Ledger projection 与磁盘内容比较，决定清理或更新 pending。
//! - 比较结果只影响 pending side table，不得直接改写 Ledger。

use super::pending;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::models::DocId;
use anyhow::Result;
use std::sync::Arc;

pub(super) fn sync_modified_pending(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    repo_path: &str,
    doc_id: DocId,
) -> Result<()> {
    let file_path = repo.local_repo_workspace_path(repo_name, repo_path)?;
    let disk_content = std::fs::read_to_string(&file_path).unwrap_or_default();
    let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)?;
    if rebuilt.content == disk_content {
        return pending::clear(repo, repo_name, repo_path);
    }
    pending::upsert(
        repo,
        repo_name,
        repo_path,
        crate::source_control::ChangeStatus::Modified,
        Some(doc_id),
        None,
    )
}
