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
        let staged = staging::list_staged_with_status(&self.local_db)?;
        if staged.is_empty() {
            anyhow::bail!("Nothing to commit: staging area is empty");
        }

        let doc_count = staged.len() as u32;

        for (path, status) in &staged {
            let normalized = to_forward_slash(path);
            match status {
                ChangeStatus::Added | ChangeStatus::Modified => {
                    self.commit_file_ops(&vault_root, &normalized)?;
                }
                ChangeStatus::Deleted => {
                    self.commit_delete_snapshot(&normalized)?;
                }
            }
        }

        // 获取追加 Op 后的最大序列号
        let ledger_seq = range::get_max_seq(&self.local_db)?;
        let commit = commits::create(&self.local_db, message, doc_count, ledger_seq)?;
        staging::clear(&self.local_db)?;

        tracing::info!("Committed {} files: {}", doc_count, message);
        Ok(commit)
    }

    /// 为单个 Added/Modified 文件生成 Op 并追加 Ledger + 保存快照
    ///
    /// **流程**: 解析 doc_id → 读磁盘内容 → 取快照(旧内容) → reconcile → 追加 Op → 保存快照
    fn commit_file_ops(&self, vault_root: &std::path::Path, normalized_path: &str) -> Result<()> {
        let doc_id = self.resolve_or_create_docid(normalized_path)?;
        let disk_path = vault_root.join(normalized_path);
        let disk_content = std::fs::read_to_string(&disk_path).unwrap_or_default(); // 文件不存在视为空

        // 获取已有 Ledger ops 用于 reconcile
        let existing_ops = self.get_local_ops(doc_id)?;
        let entries: Vec<_> = existing_ops.into_iter().map(|(_, e)| e).collect();

        // 计算需要追加的 Op (使磁盘内容与 Ledger 状态一致)
        let new_ops = reconcile::compute_reconcile_ops(doc_id, &entries, &disk_content)?;

        // 追加到 Ledger
        for op_entry in &new_ops {
            self.append_local_op(op_entry)?;
        }

        // 保存快照 (提交后的最新内容)
        changes::save_snapshot(&self.local_db, doc_id, normalized_path, &disk_content)?;
        Ok(())
    }

    /// 处理 Deleted 文件：删除快照
    fn commit_delete_snapshot(&self, normalized_path: &str) -> Result<()> {
        use crate::source_control::snapshot_paths;
        if let Some(doc_id) = snapshot_paths::find_snapshot_doc_id(&self.local_db, normalized_path)?
        {
            changes::remove_snapshot(&self.local_db, doc_id)?;
        }
        Ok(())
    }

    /// 解析路径对应的 DocId，不存在则创建
    fn resolve_or_create_docid(&self, normalized_path: &str) -> Result<crate::models::DocId> {
        use crate::ledger::metadata;
        if let Some(doc_id) = metadata::get_docid(&self.local_db, normalized_path)? {
            return Ok(doc_id);
        }
        // 新文件：创建 DocId 绑定
        metadata::create_docid(&self.local_db, normalized_path)
    }
}
