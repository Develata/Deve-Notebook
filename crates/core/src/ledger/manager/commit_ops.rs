// crates/core/src/ledger/manager/commit_ops.rs
//! # 提交时 Op 生成逻辑 (Commit-time Op Generation)
//!
//! 实现三阶段工作流的提交核心：读磁盘内容 → diff 快照 → 生成 Op → 追加 Ledger。
//!
//! **Invariant**: 只有经过 Stage 的文件才会在提交时生成 Op 并写入 Ledger。
//! **Pre-condition**: `vault_root` 已设置，暂存区非空。
//! **Post-condition**: Ledger 包含新 Op，快照已更新，提交记录已创建，暂存区已清空。

use crate::ledger::RepoManager;
use crate::ledger::range;
use crate::source_control::{ChangeStatus, CommitInfo, changes, commits, staging};
use crate::sync::reconcile;
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use std::path::PathBuf;

impl RepoManager {
    /// 三阶段工作流提交：从磁盘读取内容 → 生成 Op → 追加 Ledger → 创建提交
    ///
    /// **流程**:
    /// 1. 列出暂存文件及其变更状态
    /// 2. 对每个 Added/Modified 文件：读磁盘 → diff 快照 → 生成 Op → 追加 Ledger
    /// 3. 对每个 Deleted 文件：删除快照
    /// 4. 创建 CommitInfo (parent_id 自动推导)
    /// 5. 清空暂存区
    pub(crate) fn commit_staged_with_ops(
        &self,
        message: &str,
        vault_root: PathBuf,
    ) -> Result<CommitInfo> {
        self.commit_staged_with_ops_in_local_repo(self.local_repo_name(), message, vault_root)
    }

    pub(crate) fn commit_staged_with_ops_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
        vault_root: PathBuf,
    ) -> Result<CommitInfo> {
        let staged = self.run_on_local_repo(repo_name, staging::list_staged_with_status)?;
        if staged.is_empty() {
            anyhow::bail!("Nothing to commit: staging area is empty");
        }

        let doc_count = staged.len() as u32;
        for (path, status) in &staged {
            let normalized = to_forward_slash(path);
            match status {
                ChangeStatus::Added | ChangeStatus::Modified => {
                    self.commit_file_ops_in_local_repo(repo_name, &vault_root, &normalized)?;
                }
                ChangeStatus::Deleted => {
                    self.commit_delete_snapshot_in_local_repo(repo_name, &normalized)?;
                }
            }
        }

        let commit = self.run_on_local_repo(repo_name, |db| {
            let ledger_seq = range::get_max_seq(db)?;
            let commit = commits::create(db, message, doc_count, ledger_seq)?;
            staging::clear(db)?;
            Ok(commit)
        })?;
        tracing::info!(
            "Committed {} files in {}: {}",
            doc_count,
            repo_name,
            message
        );
        Ok(commit)
    }

    fn commit_file_ops_in_local_repo(
        &self,
        repo_name: &str,
        _vault_root: &std::path::Path,
        normalized_path: &str,
    ) -> Result<()> {
        let doc_id = self.resolve_or_create_docid_in_local_repo(repo_name, normalized_path)?;
        let disk_path = self.local_repo_workspace_path(repo_name, normalized_path)?;
        let disk_content = std::fs::read_to_string(&disk_path).unwrap_or_default();
        let existing_ops = self.get_local_ops_in_local_repo(repo_name, doc_id)?;
        let entries: Vec<_> = existing_ops.into_iter().map(|(_, e)| e).collect();
        let patch = reconcile::compute_reconcile_patch(&entries, &disk_content)?;
        reconcile::append_patch_in_local_repo(self, repo_name, doc_id, "local_commit", &patch)?;

        self.run_on_local_repo(repo_name, |db| {
            changes::save_snapshot(db, doc_id, normalized_path, &disk_content)
        })
    }

    fn commit_delete_snapshot_in_local_repo(
        &self,
        repo_name: &str,
        normalized_path: &str,
    ) -> Result<()> {
        use crate::source_control::snapshot_paths;
        self.run_on_local_repo(repo_name, |db| {
            if let Some(doc_id) = snapshot_paths::find_snapshot_doc_id(db, normalized_path)? {
                changes::remove_snapshot(db, doc_id)?;
            }
            Ok(())
        })
    }

    fn resolve_or_create_docid_in_local_repo(
        &self,
        repo_name: &str,
        normalized_path: &str,
    ) -> Result<crate::models::DocId> {
        if let Some(doc_id) = self.get_docid_in_local_repo(repo_name, normalized_path)? {
            return Ok(doc_id);
        }
        self.create_docid_in_local_repo(repo_name, normalized_path)
    }
}
