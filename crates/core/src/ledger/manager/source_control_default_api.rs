//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!
//! # Source Control 默认 Repo 兼容入口
//!
//! 把历史单 repo 便捷方法与 repo-scoped 执行路径隔离。

use crate::config::GitBridgeMode;
use crate::ledger::RepoManager;
use crate::source_control::{ChangeEntry, CommitInfo};
use anyhow::Result;

impl RepoManager {
    /// 取消暂存指定文件
    pub fn unstage_file(&self, path: &str) -> Result<()> {
        self.unstage_file_in_local_repo(self.local_repo_name(), path)
    }

    /// 获取已暂存文件列表
    pub fn list_staged(&self) -> Result<Vec<ChangeEntry>> {
        self.list_staged_in_local_repo(self.local_repo_name())
    }

    /// 获取提交历史
    pub fn list_commits(&self, limit: u32) -> Result<Vec<CommitInfo>> {
        self.list_commits_in_local_repo(self.local_repo_name(), limit)
    }

    /// 提交已暂存文件（三阶段工作流的唯一入口）
    ///
    /// **流程**: 读取暂存文件 → 读磁盘内容 → diff 快照 → 生成 Op → 追加 Ledger → 快照更新 → 创建提交
    /// **Invariant**: repo Projection Locator 必须存在；不存在即为配置错误。
    pub fn commit_staged(&self, message: &str) -> Result<CommitInfo> {
        self.commit_staged_in_local_repo(self.local_repo_name(), message)
    }

    pub fn commit_staged_with_git_bridge(
        &self,
        message: &str,
        git_bridge: GitBridgeMode,
    ) -> Result<CommitInfo> {
        self.commit_staged_in_local_repo_with_git_bridge(
            self.local_repo_name(),
            message,
            git_bridge,
        )
    }

    /// 获取所有待确认的文件变更
    pub fn list_pending_fs(&self) -> Result<Vec<ChangeEntry>> {
        self.list_pending_fs_in_local_repo(self.local_repo_name())
    }

    /// 将待确认变更移入暂存区 (Working Dir → Staging)
    pub fn stage_pending(&self, path: &str) -> Result<()> {
        self.stage_pending_in_local_repo(self.local_repo_name(), path)
    }

    /// 丢弃待确认变更
    pub fn discard_pending(&self, path: &str) -> Result<()> {
        self.discard_pending_in_local_repo(self.local_repo_name(), path)
    }
}
