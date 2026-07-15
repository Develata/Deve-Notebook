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
use crate::protocol::ScPathTarget;
use crate::source_control::{
    CommitAuthorityFailure, CommitInfo, ExternalApplyOutcome, ExternalApplyReceipt,
    PreparedExternalApply,
};
use anyhow::Result;

impl RepoManager {
    pub fn unstage_file_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.source_control_runtime()
            .unstage_file_in_local_repo(repo_name, path)
    }

    pub fn commit_source_control_changes_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> Result<CommitInfo> {
        self.source_control_runtime()
            .commit_source_control_changes_in_local_repo(repo_name, message)
    }

    /// Authority-only server entry point. Git mirror inspection and queueing
    /// must run after the caller releases its repository mutation permit.
    pub fn commit_source_control_authority_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> std::result::Result<CommitInfo, CommitAuthorityFailure> {
        self.commit_runtime()
            .commit_source_control_authority_in_local_repo(repo_name, message)
    }

    pub fn prepare_source_control_commit_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Option<PreparedExternalApply>> {
        self.commit_runtime()
            .prepare_source_control_commit_in_local_repo(repo_name)
    }

    pub fn commit_source_control_authority_with_prepared_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
        prepared_external: Option<PreparedExternalApply>,
    ) -> std::result::Result<CommitInfo, CommitAuthorityFailure> {
        self.commit_runtime()
            .commit_source_control_authority_with_prepared_in_local_repo(
                repo_name,
                message,
                prepared_external,
            )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn enqueue_git_mirror_projection_in_local_repo(
        &self,
        repo_name: &str,
        expected_repo_id: crate::models::RepoId,
        commit: &CommitInfo,
    ) {
        self.commit_runtime()
            .enqueue_git_mirror_projection_in_local_repo(repo_name, expected_repo_id, commit);
    }

    pub fn apply_external_changes_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<ExternalApplyReceipt> {
        self.source_control_runtime()
            .apply_external_changes_in_local_repo(repo_name)
    }

    pub fn prepare_external_changes_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<PreparedExternalApply> {
        self.commit_runtime()
            .prepare_external_changes_in_local_repo(repo_name)
    }

    pub fn commit_prepared_external_changes_in_local_repo(
        &self,
        repo_name: &str,
        prepared: PreparedExternalApply,
    ) -> Result<ExternalApplyOutcome> {
        self.commit_runtime()
            .commit_prepared_external_changes_in_local_repo(repo_name, prepared)
    }

    // === Pending FS Ops (Working Directory) ===

    pub fn stage_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.source_control_runtime()
            .stage_pending_in_local_repo(repo_name, path)
    }

    pub fn stage_pending_targets_in_local_repo(
        &self,
        repo_name: &str,
        targets: &[ScPathTarget],
    ) -> Result<()> {
        self.source_control_runtime()
            .stage_pending_targets_in_local_repo(repo_name, targets)
    }

    pub fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        self.source_control_runtime()
            .discard_pending_in_local_repo(repo_name, path)
    }
}
