// apps/cli/src/commands/watch.rs
//! # Watch 命令
//!
//! 启动文件系统监听，实时捕获变更并同步到 Ledger。

use anyhow::Result;
use deve_core::ledger::RepoManager;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 全局运行标志 (用于 Ctrl+C 信号处理)
static RUNNING: AtomicBool = AtomicBool::new(true);

/// 监控命令入口
///
/// **功能**:
/// 启动文件系统监听，实时捕获变更并同步到 `ledger`。
/// 组合 `RepoManager`, `SyncManager` 和 `Watcher`。
///
/// **阻塞行为**:
/// 此函数会阻塞直到收到 Ctrl+C 信号。
pub fn run(
    ledger_dir: &Path,
    vault_path: &Path,
    snapshot_depth: usize,
    dry_run: bool,
) -> Result<()> {
    // 1. 初始化 RepoManager
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root_checked(vault_path)?;
    let repo = Arc::new(repo);

    // 2. 初始化 SyncManager
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new_checked(
        repo.clone(),
        vault_path.to_path_buf(),
    )?);
    if dry_run {
        repo.list_local_repo_names_for_execution()?;
        println!("Watcher dry-run OK: {:?}", vault_path);
        return Ok(());
    }
    let repo_ids = repo
        .list_local_repo_names_for_execution()?
        .into_iter()
        .map(|repo_name| {
            deve_core::sync::watcher::start_repo_watcher(
                sync_manager.clone(),
                &repo_name,
                None,
                None,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    // 3. 注册 Ctrl+C 信号处理 (必须在 watcher.watch() 之前)
    ctrlc::set_handler(move || {
        println!("\n收到退出信号，正在停止...");
        RUNNING.store(false, Ordering::SeqCst);
    })
    .expect("无法设置 Ctrl+C 处理器");

    // 4. 创建并启动 Watcher
    println!("启动 Watcher: {:?}", vault_path);
    println!("按 Ctrl+C 停止...");

    // 5. 阻塞主线程直到收到退出信号
    while RUNNING.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    for repo_id in repo_ids {
        deve_core::sync::watcher::stop_repo_watcher(repo_id)?;
    }
    println!("Watcher 已停止。");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    use tempfile::TempDir;

    #[test]
    fn watch_dry_run_returns_without_blocking() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let vault_dir = dir.path().join("vault");
        std::fs::create_dir_all(&vault_dir).expect("create vault");

        run(&ledger_dir, &vault_dir, 8, true).expect("watch dry-run");
    }
}
