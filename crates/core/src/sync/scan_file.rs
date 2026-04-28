//! # 全量扫描单文件处理
//! plan_ref:
//!   - 04_storage#watcher-contract
//!   - 07_diff_logic#source-control-runtime
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
use std::sync::Arc;

pub(super) fn scan_disk_file(
    repo: &Arc<RepoManager>,
    vfs: &Vfs,
    repo_name: &str,
    repo_path: &str,
) -> Result<Option<DocId>> {
    let root_rel = repo.local_repo_workspace_relative(repo_name, repo_path);
    let inode = vfs.get_inode(&root_rel)?;
    if let Some(inode) = inode
        && let Some(doc_id) = repo.get_docid_by_inode_in_local_repo(repo_name, &inode)?
    {
        let meta = repo.get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?;
        if meta.as_ref().map(|item| item.path.as_str()) != Some(repo_path)
            && let Some(old_path) = meta.map(|item| item.path)
        {
            pending_rename::upsert_external_rename(repo, repo_name, &old_path, repo_path, doc_id)?;
            return Ok(Some(doc_id));
        }
        repo.bind_inode_in_local_repo(repo_name, &inode, doc_id)?;
        let _ = pending_content::sync_modified_pending(repo, repo_name, repo_path, doc_id)?;
        return Ok(Some(doc_id));
    }

    let Some(doc_id) = repo.get_tracked_docid_in_local_repo(repo_name, repo_path)? else {
        pending::upsert(repo, repo_name, repo_path, ChangeStatus::Added, None, None)?;
        return Ok(None);
    };
    if let Some(inode) = inode {
        repo.bind_inode_in_local_repo(repo_name, &inode, doc_id)?;
    }
    let _ = pending_content::sync_modified_pending(repo, repo_name, repo_path, doc_id)?;
    Ok(Some(doc_id))
}
