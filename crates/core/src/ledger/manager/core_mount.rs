use anyhow::Result;
use std::path::Path;

use crate::ledger::manager::types::RepoManager;

impl RepoManager {
    /// 设置 Vault 根目录并强制校验本地 catalog。
    ///
    /// Invariants:
    /// - 生产态在挂载 Vault 后，必须立即暴露本地 repo catalog 损坏。
    /// - 测试辅助可继续使用 `set_vault_root` 的宽松包装。
    pub fn set_vault_root_checked(&mut self, root: impl AsRef<Path>) -> Result<()> {
        let previous_root = self.vault_root.clone();
        self.vault_root = Some(root.as_ref().to_path_buf());
        if let Err(err) = self.refresh_local_repo_catalog() {
            self.vault_root = previous_root;
            return Err(err);
        }
        Ok(())
    }

    /// 设置 Vault 根目录——宽松包装，仅供测试辅助使用。
    ///
    /// 生产代码必须使用 `set_vault_root_checked`，它会在 catalog 校验失败时返回错误。
    /// 本方法会在目录缺失时尝试创建 Vault，以兼容测试辅助。
    /// 若仅 catalog 校验失败，则仍保留请求的 Vault root，供后续显式 repair 使用。
    /// 本方法无法标记 `#[cfg(test)]`，因为下游 crate 的测试模块也会调用它。
    pub fn set_vault_root(&mut self, root: impl AsRef<Path>) {
        let previous_root = self.vault_root.clone();
        let requested_root = root.as_ref().to_path_buf();
        if let Err(error) = std::fs::create_dir_all(&requested_root) {
            self.vault_root = previous_root;
            tracing::warn!(
                "Failed to create vault root before mounting {:?}: {}",
                requested_root,
                error
            );
            return;
        }
        if let Err(error) = self.set_vault_root_checked(root) {
            self.vault_root = previous_root.or(Some(requested_root));
            tracing::warn!("Failed to repair local repo catalog after mounting vault: {error}");
        }
    }
}
