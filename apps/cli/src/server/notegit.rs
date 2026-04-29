use anyhow::Result;
use deve_core::ledger::RepoManager;
use std::path::{Path, PathBuf};

/// 准备 host-scoped 与 repo-scoped `.notegit` 目录。
///
/// Invariants:
/// - 运行时只接受计划定义的 `ledger/.host` 与 `vault/<repo>/.notegit/` 布局。
/// - 启动路径不再接受旧 `.deve` 或 Vault 根目录元数据。
pub fn prepare(repo: &RepoManager, _vault_root: &Path) -> Result<PathBuf> {
    let host_keys_dir = deve_core::utils::notegit::host_keys_dir(repo.ledger_dir());
    let main_repo_root = repo.local_repo_workspace_root(repo.local_repo_name())?;
    std::fs::create_dir_all(&host_keys_dir)?;
    std::fs::create_dir_all(deve_core::utils::notegit::repo_keys_dir(&main_repo_root))?;
    std::fs::create_dir_all(deve_core::utils::notegit::repo_dir(&main_repo_root))?;
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&main_repo_root)?;
    Ok(host_keys_dir)
}
