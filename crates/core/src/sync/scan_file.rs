//! # 全量扫描单文件处理
//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 05_diff_logic#source-control-runtime
//!
//! Invariants:
//! - 全量扫描必须优先使用 inode -> doc_id 维持稳定身份。
//! - rename 候选只能写入 pending side table，不得在扫描阶段改写 projection 真值。

use super::pending;
use super::pending_content;
use super::pending_rename;
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::ChangeStatus;
use crate::vfs::Vfs;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

pub(super) fn scan_disk_file(
    repo: &Arc<RepoManager>,
    vfs: &Vfs,
    repo_name: &str,
    repo_path: &str,
    disk_path: &Path,
) -> Result<Option<DocId>> {
    let inode = vfs.get_inode_abs(disk_path)?;
    if let Some(inode) = inode
        && let Some(doc_id) = repo.get_docid_by_inode_in_local_repo(repo_name, &inode)?
    {
        let meta = repo.get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?;
        if meta.as_ref().map(|item| item.path.as_str()) != Some(repo_path)
            && let Some(old_path) = meta.map(|item| item.path)
        {
            pending_rename::upsert_external_rename_at_path(
                repo, repo_name, &old_path, repo_path, disk_path, doc_id,
            )?;
            return Ok(Some(doc_id));
        }
        repo.bind_inode_in_local_repo(repo_name, &inode, doc_id)?;
        let _ = pending_content::sync_modified_pending_at_path(
            repo, repo_name, repo_path, doc_id, disk_path,
        )?;
        return Ok(Some(doc_id));
    }

    let Some(doc_id) = repo.get_tracked_docid_in_local_repo(repo_name, repo_path)? else {
        pending::upsert_from_disk_path(
            repo,
            repo_name,
            repo_path,
            ChangeStatus::Added,
            None,
            None,
            disk_path,
        )?;
        return Ok(None);
    };
    if let Some(inode) = inode {
        repo.bind_inode_in_local_repo(repo_name, &inode, doc_id)?;
    }
    let _ = pending_content::sync_modified_pending_at_path(
        repo, repo_name, repo_path, doc_id, disk_path,
    )?;
    Ok(Some(doc_id))
}
