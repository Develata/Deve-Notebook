// apps/cli/src/commands/watch.rs
//! # Watch 命令
//!
//! 启动文件系统监听，实时捕获变更并同步到 Ledger。

use anyhow::Result;
use deve_core::ledger::RepoManager;
use std::path::PathBuf;
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
pub fn run(ledger_dir: &PathBuf, vault_path: &PathBuf, snapshot_depth: usize) -> Result<()> {
    // 1. 初始化 RepoManager
    let repo = Arc::new(RepoManager::init(ledger_dir, snapshot_depth, None, None)?);

    // 2. 初始化 SyncManager
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(
        repo.clone(),
        vault_path.clone(),
    ));

    // 3. 创建并启动 Watcher
    let watcher = deve_core::watcher::Watcher::new(sync_manager, vault_path.clone());
    println!("启动 Watcher: {:?}", vault_path);
    println!("按 Ctrl+C 停止...");
    watcher.watch()?;

    // 4. 注册 Ctrl+C 信号处理
    ctrlc::set_handler(move || {
        println!("\n收到退出信号，正在停止...");
        RUNNING.store(false, Ordering::SeqCst);
    })
    .expect("无法设置 Ctrl+C 处理器");

    // 5. 阻塞主线程直到收到退出信号
    while RUNNING.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("Watcher 已停止。");
    Ok(())
}
