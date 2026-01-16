use std::path::PathBuf;
use std::sync::Arc;
use deve_core::ledger::RepoManager;
use anyhow::Result;

/// 监控命令
///
/// **功能**:
/// 启动文件系统监听，实时捕获变更并同步到 `ledger`。
/// 组合 `RepoManager`, `SyncManager` 和 `Watcher`。
pub fn run(ledger_dir: &PathBuf, vault_path: &PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    // 初始化 RepoManager
    let repo = Arc::new(RepoManager::init(ledger_dir, snapshot_depth, None)?);
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault_path.clone()));
    let watcher = deve_core::watcher::Watcher::new(sync_manager, vault_path.clone());
    println!("Starting watcher on {:?}... Press Ctrl+C to stop.", vault_path);
    watcher.watch()?;
    Ok(())
}
