// apps/cli/src/commands/watch.rs
//! # Watch 命令
//! plan_ref:
//!   - 03_storage#watcher-contract
//!   - 14_commands#cli-commands
//!
//! 启动文件系统监听，实时捕获变更并同步到 Ledger。

use anyhow::{Context, Result};
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
pub fn run(ledger_dir: &Path, snapshot_depth: usize, dry_run: bool) -> Result<()> {
    // 1. 初始化 RepoManager
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo = Arc::new(repo);

    // 2. 初始化 SyncManager
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?);
    if dry_run {
        repo.list_local_repo_names_for_execution()?;
        println!("Watcher dry-run OK: repo projection workspaces resolved");
        return Ok(());
    }
    install_shutdown_handler()?;
    sync_manager.scan()?;
    let repo_ids = sync_manager
        .healthy_local_repo_names_for_execution()?
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

    // 4. 创建并启动 Watcher
    println!("启动 Watcher: repo projection workspaces");
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

fn install_shutdown_handler() -> Result<()> {
    ctrlc::set_handler(shutdown_signal_handler()).context("Failed to set Ctrl-C handler")
}

fn shutdown_signal_handler() -> impl FnMut() + Send + 'static {
    || {
        println!("\n收到退出信号，正在停止...");
        RUNNING.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::{RUNNING, run, shutdown_signal_handler};
    use std::sync::atomic::Ordering;
    use tempfile::TempDir;

    #[test]
    fn watch_dry_run_returns_without_blocking() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        std::fs::create_dir_all(&projection_base).expect("create projection base");

        let repo = deve_core::ledger::RepoManager::init(&ledger_dir, 8, Some("default"), None)
            .expect("init");
        repo.set_projection_base_for_local_repo("default", &projection_base)
            .expect("locator");

        run(&ledger_dir, 8, true).expect("watch dry-run");
    }

    #[test]
    fn shutdown_signal_handler_marks_watch_loop_stopped() {
        RUNNING.store(true, Ordering::SeqCst);
        let mut handler = shutdown_signal_handler();

        handler();

        assert!(!RUNNING.load(Ordering::SeqCst));
        RUNNING.store(true, Ordering::SeqCst);
    }
}
