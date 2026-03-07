use crate::ledger::RepoManager;
use crate::models::RepoId;
use crate::utils::path::{join_normalized, to_forward_slash};
use anyhow::{Result, anyhow};
use std::path::PathBuf;

impl RepoManager {
    /// 返回指定本地 repo 在 Vault 下的物理根目录：`vault/<repo_name>/`
    pub fn local_repo_workspace_root(&self, repo_name: &str) -> Result<PathBuf> {
        let vault_root = self
            .vault_root
            .as_ref()
            .ok_or_else(|| anyhow!("Vault root is not configured"))?;
        Ok(join_normalized(
            vault_root,
            repo_name.trim_end_matches(".redb"),
        ))
    }

    /// 返回指定本地 repo 下某个文档的物理路径：`vault/<repo_name>/<repo_path>`
    pub fn local_repo_workspace_path(&self, repo_name: &str, repo_path: &str) -> Result<PathBuf> {
        let repo_root = self.local_repo_workspace_root(repo_name)?;
        if repo_path.is_empty() {
            return Ok(repo_root);
        }
        Ok(join_normalized(&repo_root, repo_path))
    }

    /// 构造 Watcher / PersistGuard 使用的 Vault 根相对路径。
    pub fn local_repo_workspace_relative(&self, repo_name: &str, repo_path: &str) -> String {
        let repo_name = repo_name.trim_end_matches(".redb");
        let repo_path = to_forward_slash(repo_path).trim_matches('/').to_string();
        if repo_path.is_empty() {
            repo_name.to_string()
        } else {
            format!("{repo_name}/{repo_path}")
        }
    }

    /// 将 `vault/<repo_name>/<repo_path>` 形式的根相对路径解析回本地 repo 作用域。
    ///
    /// 返回 `(repo_name, repo_id, repo_path)`；若不在本地 repo 目录下则返回 `None`。
    pub fn resolve_local_workspace_path(
        &self,
        root_relative: &str,
    ) -> Result<Option<(String, RepoId, String)>> {
        let normalized = to_forward_slash(root_relative)
            .trim_matches('/')
            .to_string();
        if normalized.is_empty() {
            return Ok(None);
        }

        let (repo_name, repo_path) = match normalized.split_once('/') {
            Some((repo_name, repo_path)) => (repo_name, repo_path),
            None => (normalized.as_str(), ""),
        };

        if repo_name.is_empty() || repo_name.starts_with('.') {
            return Ok(None);
        }

        let Some(info) = self.get_repo_info_for(None, Some(repo_name))? else {
            return Ok(None);
        };

        Ok(Some((info.name, info.uuid, repo_path.to_string())))
    }
}
