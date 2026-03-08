// crates/core/src/ledger/manager/source_control_ops.rs
//! # 版本控制集成
//!
//! 实现 `RepoManager` 的暂存、提交、历史等版本控制方法。

use crate::ledger::RepoManager;
use crate::ledger::source_control;
use crate::models::DocId;
use crate::source_control::snapshot_paths;
use crate::source_control::{ChangeEntry, ChangeStatus, CommitInfo, SnapshotUpdate, pending_fs};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

impl RepoManager {
    /// 暂存指定文件
    pub fn stage_file(&self, path: &str) -> Result<()> {
        self.stage_file_in_local_repo(self.local_repo_name(), path)
    }

    pub fn stage_file_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.stage_pending_in_local_repo(repo_name, path)
    }

    /// 取消暂存指定文件
    pub fn unstage_file(&self, path: &str) -> Result<()> {
        self.unstage_file_in_local_repo(self.local_repo_name(), path)
    }

    pub fn unstage_file_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| source_control::unstage_file(db, path))
    }

    /// 获取已暂存文件列表
    pub fn list_staged(&self) -> Result<Vec<ChangeEntry>> {
        self.list_staged_in_local_repo(self.local_repo_name())
    }

    pub fn list_staged_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.run_on_local_repo(repo_name, source_control::list_staged)
    }

    /// 创建提交 (保存快照)
    pub fn create_commit_with_snapshots<F>(
        &self,
        message: &str,
        get_content: F,
    ) -> Result<CommitInfo>
    where
        F: Fn(&str) -> Option<(DocId, String)>,
    {
        source_control::create_commit(&self.local_db, message, get_content)
    }

    /// 获取提交历史
    pub fn list_commits(&self, limit: u32) -> Result<Vec<CommitInfo>> {
        source_control::list_commits(&self.local_db, limit)
    }

    /// 获取文档的已提交内容 (用于 Diff)
    pub fn get_committed_content(&self, doc_id: DocId) -> Result<Option<String>> {
        source_control::get_committed_content(&self.local_db, doc_id)
    }

    /// 检测单个文档的变更状态
    pub fn detect_change(
        &self,
        committed: Option<&str>,
        current: Option<&str>,
    ) -> Option<ChangeStatus> {
        source_control::detect_change(committed, current)
    }

    /// 提交已暂存文件 (新三阶段工作流)
    ///
    /// **流程**: 读取暂存文件 → 读磁盘内容 → diff 快照 → 生成 Op → 追加 Ledger → 快照更新 → 创建提交
    /// **Invariant**: vault_root 必须已设置，否则回退到旧逻辑。
    pub fn commit_staged(&self, message: &str) -> Result<CommitInfo> {
        self.commit_staged_in_local_repo(self.local_repo_name(), message)
    }

    pub fn commit_staged_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> Result<CommitInfo> {
        if let Some(vault_root) = &self.vault_root {
            if repo_name == self.local_repo_name() {
                return self.commit_staged_with_ops(message, vault_root.clone());
            }
            return self.commit_staged_with_ops_in_local_repo(
                repo_name,
                message,
                vault_root.clone(),
            );
        }
        // 回退：旧逻辑 (从 Ledger 重建内容)
        self.commit_staged_legacy_in_local_repo(repo_name, message)
    }

    /// 旧版提交逻辑 (从 Ledger 重建内容，兼容用)
    fn commit_staged_legacy_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> Result<CommitInfo> {
        self.run_on_local_repo(repo_name, |db| {
            source_control::create_commit_with_updates(db, message, |path| {
                let normalized = to_forward_slash(path);
                if let Ok(Some(doc_id)) = crate::ledger::metadata::get_docid(db, &normalized) {
                    let ops = crate::ledger::ops::get_ops_from_db(db, doc_id).ok()?;
                    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
                    let content = crate::state::reconstruct_content(&entries);
                    Some(SnapshotUpdate::Save {
                        doc_id,
                        path: normalized,
                        content,
                    })
                } else if let Ok(Some(doc_id)) =
                    snapshot_paths::find_snapshot_doc_id(db, &normalized)
                {
                    Some(SnapshotUpdate::Delete { doc_id })
                } else {
                    None
                }
            })
        })
    }

    // === Pending FS Ops (Working Directory) ===

    /// 获取所有待确认的文件变更
    pub fn list_pending_fs(&self) -> Result<Vec<ChangeEntry>> {
        self.list_pending_fs_in_local_repo(self.local_repo_name())
    }

    pub fn list_pending_fs_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.run_on_local_repo(repo_name, |db| {
            let entries = pending_fs::list_all(db)?;
            Ok(entries
                .into_iter()
                .map(|e| ChangeEntry {
                    path: e.path,
                    status: e.change_type,
                    has_conflict: e.has_conflict,
                })
                .collect())
        })
    }

    /// 将待确认变更移入暂存区 (Working Dir → Staging)
    ///
    /// **流程**: 读取 pending 状态 → pending_fs_ops 中移除 → staged_files 中插入（带状态）
    pub fn stage_pending(&self, path: &str) -> Result<()> {
        self.stage_pending_in_local_repo(self.local_repo_name(), path)
    }

    pub fn stage_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| {
            let status = pending_fs::get(db, path)?
                .map(|e| e.change_type)
                .unwrap_or(ChangeStatus::Modified);
            pending_fs::remove(db, path)?;
            source_control::stage_file_with_status(db, path, status)?;
            Ok(())
        })
    }

    /// 丢弃待确认变更
    pub fn discard_pending(&self, path: &str) -> Result<()> {
        self.discard_pending_in_local_repo(self.local_repo_name(), path)
    }

    pub fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| pending_fs::remove(db, path))
    }
}
