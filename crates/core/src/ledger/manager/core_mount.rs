//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 04_storage#projection-locator-contract
//!
use anyhow::Result;
use std::path::Path;

use crate::ledger::manager::types::RepoManager;

impl RepoManager {
    /// 为所有当前本地 repo 设置同一个 Projection Locator base。
    ///
    /// Invariants:
    /// - 参数是 projection base；最终 workspace root 仍为 `<base>/<repo_name>/`。
    /// - 生产入口应优先通过 `set_projection_base_for_local_repo` 明确绑定目标 repo。
    pub fn set_projection_base_for_all_local_repos_checked(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<()> {
        self.refresh_local_repo_catalog()?;
        for repo_name in self.list_local_repo_names_for_execution()? {
            self.set_projection_base_for_local_repo(&repo_name, root.as_ref())?;
        }
        Ok(())
    }
}
