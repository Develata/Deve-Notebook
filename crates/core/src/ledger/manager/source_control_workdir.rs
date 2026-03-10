//! # Source Control 工作区辅助
//!
//! Invariants:
//! - Discard 只能恢复工作区到当前 Ledger 投影，不能改写 Ledger。
//! - 外部 Working Directory diff 的左侧永远来自当前 Ledger 投影，右侧来自磁盘文件。

use crate::ledger::RepoManager;
use crate::ledger::metadata;
use crate::models::DocId;
use crate::source_control::{ChangeStatus, pending_fs, snapshot_paths};
use crate::state::reconstruct_content;
use crate::utils::path::to_forward_slash;
use anyhow::Result;

impl RepoManager {
    pub fn workdir_diff_inputs_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<(String, String)> {
        let normalized = to_forward_slash(path);
        let old_content = match self.resolve_workdir_doc_id_in_local_repo(repo_name, &normalized)? {
            Some(doc_id) => rebuild_doc_projection(self, repo_name, doc_id)?,
            None => String::new(),
        };
        let file_path = self.local_repo_workspace_path(repo_name, &normalized)?;
        let new_content = if file_path.exists() {
            std::fs::read_to_string(file_path)?
        } else {
            String::new()
        };
        if old_content.is_empty() && new_content.is_empty() {
            anyhow::bail!("Doc not found: {}", normalized);
        }
        Ok((old_content, new_content))
    }

    pub(crate) fn resolve_workdir_doc_id_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<Option<DocId>> {
        self.run_on_local_repo(repo_name, |db| {
            let by_path = metadata::get_docid(db, path)?;
            if by_path.is_some() {
                return Ok(by_path);
            }
            snapshot_paths::find_snapshot_doc_id(db, path)
        })
    }

    pub(crate) fn discard_pending_workdir_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<()> {
        let normalized = to_forward_slash(path);
        let entry = self
            .run_on_local_repo(repo_name, |db| pending_fs::get(db, &normalized))?
            .ok_or_else(|| anyhow::anyhow!("Path is not in pending_fs_ops: {}", normalized))?;

        match entry.change_type {
            ChangeStatus::Added => discard_added(self, repo_name, &normalized)?,
            ChangeStatus::Modified | ChangeStatus::Deleted => {
                let doc_id = self
                    .resolve_workdir_doc_id_in_local_repo(repo_name, &normalized)?
                    .ok_or_else(|| anyhow::anyhow!("Document not found: {}", normalized))?;
                let content = rebuild_doc_projection(self, repo_name, doc_id)?;
                let file_path = self.local_repo_workspace_path(repo_name, &normalized)?;
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(file_path, content)?;
                self.run_on_local_repo(repo_name, |db| {
                    metadata::set_doc_path(db, doc_id, &normalized)?;
                    pending_fs::remove(db, &normalized)
                })?;
            }
        }

        Ok(())
    }
}

fn rebuild_doc_projection(repo: &RepoManager, repo_name: &str, doc_id: DocId) -> Result<String> {
    let ops = repo.get_local_ops_in_local_repo(repo_name, doc_id)?;
    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
    Ok(reconstruct_content(&entries))
}

fn discard_added(repo: &RepoManager, repo_name: &str, path: &str) -> Result<()> {
    let file_path = repo.local_repo_workspace_path(repo_name, path)?;
    if file_path.exists() {
        std::fs::remove_file(&file_path)?;
    }
    repo.run_on_local_repo(repo_name, |db| {
        pending_fs::remove(db, path)?;
        if let Some(doc_id) = metadata::get_docid(db, path)?
            && crate::source_control::changes::get_committed_content(db, doc_id)?.is_none()
        {
            metadata::delete_doc(db, path)?;
        }
        Ok(())
    })
}
