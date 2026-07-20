// apps/cli/src/commands/watch.rs
//! # Watch 命令
//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 14_commands#cli-commands
//!
//! 启动文件系统监听，将外部变更归一化为待确认文件系统 candidate。

use crate::watcher_runtime::OwnedWatcherHandles;
use anyhow::{Context, Result};
use deve_core::ledger::RepoManager;
use deve_core::sync::watcher::{RepoWatcherStart, WatcherFailure};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 全局运行标志 (用于 Ctrl+C 信号处理)
static RUNNING: AtomicBool = AtomicBool::new(true);

/// 监控命令入口
///
/// **功能**:
/// 启动文件系统监听，将外部变更写入 `pending_fs_ops`；不会直接写入 Ledger。
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
    RUNNING.store(true, Ordering::SeqCst);
    install_shutdown_handler()?;
    sync_manager.scan()?;
    let starts = sync_manager
        .healthy_local_repo_names_for_execution()?
        .into_iter()
        .map(|repo_name| RepoWatcherStart::resolve(sync_manager.clone(), repo_name, 1))
        .collect::<Result<Vec<_>, _>>()?;
    let handles = OwnedWatcherHandles::start_all(starts)?;

    // 4. 创建并启动 Watcher
    println!("启动 Watcher: repo projection workspaces");
    println!("按 Ctrl+C 停止...");

    // 5. 阻塞主线程直到收到退出信号
    let terminal_failure = loop {
        if !RUNNING.load(Ordering::SeqCst) {
            break None;
        }
        if let Some(failure) = handles.wait_terminal_failure(std::time::Duration::from_millis(100))
        {
            break Some(failure);
        }
    };

    let shutdown_result = handles.shutdown();
    finish_watch_run(terminal_failure, shutdown_result)?;
    println!("Watcher 已停止。");
    Ok(())
}

fn finish_watch_run<E>(
    terminal_failure: Option<WatcherFailure>,
    shutdown_result: Result<(), E>,
) -> Result<()>
where
    E: std::error::Error + Send + Sync + 'static,
{
    if let Some(primary) = terminal_failure {
        let mut error = anyhow::Error::new(primary);
        if let Err(shutdown) = shutdown_result {
            error = error.context(format!("watcher shutdown also failed: {shutdown}"));
        }
        return Err(error);
    }
    shutdown_result?;
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
    use super::{RUNNING, finish_watch_run, run, shutdown_signal_handler};
    use deve_core::sync::watcher::{WatcherFailure, WatcherFailureKind, WatcherFailurePhase};
    use std::sync::atomic::Ordering;
    use tempfile::TempDir;

    #[test]
    fn watch_dry_run_returns_without_blocking() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        crate::test_support::init_cataloged_repo(&ledger_dir, &projection_base, 8)
            .expect("init repo");

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

    #[test]
    fn standalone_watch_terminal_failure_returns_nonzero() {
        let failure = WatcherFailure {
            phase: WatcherFailurePhase::Receive,
            kind: WatcherFailureKind::Backend,
            primary: "terminal backend failure".to_owned(),
            cleanup: Vec::new(),
        };

        let result = finish_watch_run(Some(failure), Ok::<(), std::io::Error>(()));

        assert!(result.is_err());
        assert!(
            result
                .expect_err("terminal failure must escape")
                .to_string()
                .contains("terminal backend failure")
        );
    }
}
