// apps\cli\src\commands
//! plan_ref:
//!   - 03_storage#watcher-contract
//!   - 14_commands#cli-commands

use deve_core::ledger::RepoManager;
use std::path::Path;
use std::sync::Arc;

/// 扫描命令
///
/// **功能**:
/// 遍历 repo projection workspace，将所有 Markdown 文件注册到 `ledger` 中。
/// 使用 `Vfs` 模块执行扫描操作。
pub fn run(ledger_dir: &Path, snapshot_depth: usize) -> anyhow::Result<()> {
    // 初始化 RepoManager
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo = Arc::new(repo);
    let sync_manager = deve_core::sync::SyncManager::new_checked(repo)?;
    println!("Scanning repo projection workspaces...");
    sync_manager.scan()?;
    println!("Scanned repo-scoped workspaces successfully.");
    Ok(())
}
