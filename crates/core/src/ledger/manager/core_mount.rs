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

    /// 测试辅助：为所有本地 repo 写入 locator。
    ///
    /// 本方法无法标记 `#[cfg(test)]`，因为下游 crate 的测试模块也会调用它。
    pub fn set_projection_base_for_all_local_repos(&mut self, root: impl AsRef<Path>) {
        let requested_root = root.as_ref().to_path_buf();
        if let Err(error) = std::fs::create_dir_all(&requested_root) {
            tracing::warn!(
                "Failed to create projection base before writing locator {:?}: {}",
                requested_root,
                error
            );
            return;
        }
        if let Err(error) = self.set_projection_base_for_all_local_repos_checked(root) {
            tracing::warn!("Failed to write Projection Locator: {error}");
        }
    }
}
