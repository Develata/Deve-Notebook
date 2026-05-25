// crates/core/src/ledger/manager/source_control_ops.rs
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 03_storage/watcher#watcher-contract
//!
//! # 版本控制集成
//!
//! 实现 `RepoManager` 的 repo-scoped 暂存、提交、丢弃等写路径方法。

use crate::ledger::RepoManager;
use crate::source_control::CommitInfo;
use anyhow::Result;

impl RepoManager {
    pub fn unstage_file_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.source_control_runtime()
            .unstage_file_in_local_repo(repo_name, path)
    }

    pub fn commit_staged_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> Result<CommitInfo> {
        self.source_control_runtime()
            .commit_staged_in_local_repo(repo_name, message)
    }

    // === Pending FS Ops (Working Directory) ===

    pub fn stage_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.source_control_runtime()
            .stage_pending_in_local_repo(repo_name, path)
    }

    pub fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.source_control_runtime()
            .discard_pending_in_local_repo(repo_name, path)
    }
}
