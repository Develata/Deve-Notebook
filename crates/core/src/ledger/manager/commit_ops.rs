// crates/core/src/ledger/manager/commit_ops.rs
//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 04_storage#facts-partition
//!   - 03_rendering#document-authority-bridge
//!   - 04_storage#projection-contract
//!
//! # 提交时 Ledger Facts 生成逻辑
//!
//! 实现三阶段工作流的提交核心：读磁盘内容 → 对比快照 → 生成 Ledger Facts → 追加 Ledger。
//!
//! **Invariant**: 只有经过 Stage 的文件才会在提交时生成 Ledger Facts 并写入 Ledger。
//! **Pre-condition**: `vault_root` 已设置，暂存区非空。
//! **Post-condition**: Ledger 包含新的 Content/Structure Facts，快照已更新，提交记录已创建，暂存区已清空。

use crate::ledger::RepoManager;
use crate::ledger::manager::commit_plan;
use crate::ledger::range;
use crate::source_control::{CommitInfo, commits, staging};
use crate::utils::path::to_forward_slash;
use anyhow::{Context, Result};
use std::path::PathBuf;

impl RepoManager {
    /// 三阶段工作流提交：从磁盘读取内容 → 生成 Ledger Facts → 追加 Ledger → 创建提交
    ///
    /// **流程**:
    /// 1. 列出暂存文件及其变更状态
    /// 2. 对每个 Added/Modified 文件：先生成 Structure Facts，再追加文本 Content Facts
    /// 3. 对每个 Deleted 文件：追加删除结构事实并清理快照
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
        let staged = self.run_on_local_repo(repo_name, staging::list_staged_entries)?;
        if staged.is_empty() {
            anyhow::bail!("Nothing to commit: staging area is empty");
        }
        let mut targets = commit_plan::build_targets(staged);
        targets.sort_by_key(|target| target.delete_only);
        self.preflight_staged_commit_targets(repo_name, &targets)?;

        let doc_count = targets.len() as u32;
        for target in &targets {
            if target.delete_only {
                self.commit_delete_snapshot_in_local_repo(repo_name, &target.path, target.doc_id)?;
            } else {
                self.commit_file_ops_in_local_repo(
                    repo_name,
                    &vault_root,
                    &target.path,
                    target.doc_id,
                )?;
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

    fn preflight_staged_commit_targets(
        &self,
        repo_name: &str,
        targets: &[commit_plan::CommitTarget],
    ) -> Result<()> {
        for target in targets.iter().filter(|target| !target.delete_only) {
            let disk_path = self.local_repo_workspace_path(repo_name, &target.path)?;
            std::fs::read_to_string(&disk_path)
                .with_context(|| format!("Failed to read staged workspace file {:?}", disk_path))?;
            self.preflight_staged_upsert_identity(repo_name, target)?;
        }
        Ok(())
    }

    fn preflight_staged_upsert_identity(
        &self,
        repo_name: &str,
        target: &commit_plan::CommitTarget,
    ) -> Result<()> {
        let Some(doc_id) = target.doc_id else {
            return Ok(());
        };
        if let Some(bound_doc_id) = self.get_tracked_docid_in_local_repo(repo_name, &target.path)?
            && bound_doc_id != doc_id
        {
            anyhow::bail!(
                "source control upsert target path mismatch: staged path {} is bound to {}, but staged doc is {}",
                target.path,
                bound_doc_id,
                doc_id
            );
        }
        let Some(meta) = self.get_file_meta_for_doc_in_local_repo(repo_name, doc_id)? else {
            return Ok(());
        };
        let current_path = to_forward_slash(&meta.path);
        if current_path != target.path && !target.has_rename_evidence {
            anyhow::bail!(
                "source control upsert target path mismatch: doc {} is at {}, staged path {} lacks rename evidence",
                doc_id,
                current_path,
                target.path
            );
        }
        Ok(())
    }
}
