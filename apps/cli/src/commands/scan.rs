// apps\cli\src\commands
use deve_core::ledger::RepoManager;
use std::path::Path;
use std::sync::Arc;

/// 扫描命令
///
/// **功能**:
/// 遍历 `vault` 目录，将所有 Markdown 文件注册到 `ledger` 中。
/// 使用 `Vfs` 模块执行扫描操作。
pub fn run(ledger_dir: &Path, vault_path: &Path, snapshot_depth: usize) -> anyhow::Result<()> {
    // 初始化 RepoManager
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root_checked(vault_path)?;
    let repo = Arc::new(repo);
    let sync_manager = deve_core::sync::SyncManager::new(repo, vault_path.to_path_buf());
    println!("Scanning vault at {:?}...", vault_path);
    sync_manager.scan()?;
    println!("Scanned repo-scoped workspaces successfully.");
    Ok(())
}
