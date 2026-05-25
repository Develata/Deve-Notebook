//! plan_ref:
//!   - 04_repository#repo-selector-resolution-contract

use anyhow::Result;

use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;

impl RepoManager {
    pub fn find_local_repo_name_by_id(&self, target_id: RepoId) -> Result<Option<String>> {
        self.repo_scope_runtime()
            .find_local_repo_name_by_id(target_id)
    }

    /// Invariant: 进入本地 DB 写路径前，repo selector 必须被解析为单一 repo 名称。
    pub fn resolve_local_repo_name(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        self.repo_scope_runtime()
            .resolve_local_repo_name(repo_id, repo_name)
    }

    /// Invariants:
    /// - 执行级本地 repo 解析必须保证 `RepoUUID` 与 `repo_name` 指向同一 repo。
    /// - `repo_name` 仅作为缺失 UUID 时的回退与诊断信息，不得覆盖或被 UUID 静默覆盖。
    pub fn resolve_local_repo_name_for_execution(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        self.repo_scope_runtime()
            .resolve_local_repo_name_for_execution(repo_id, repo_name)
    }
}

#[cfg(test)]
mod tests;
