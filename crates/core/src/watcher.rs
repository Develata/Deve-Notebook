// crates\core\src
//! # 文件系统监听器
//!
//! 本模块提供 `Watcher` 结构体用于监控文件系统变更。
//!
//! ## 功能
//!
//! - 防抖的文件系统事件（300ms）避免过度处理
//! - 与 `SyncManager` 集成处理变更
//! - 回调支持，用于通过 WebSocket 广播变更
//!
//! 监听器在阻塞线程中运行，并将事件转发给同步管理器。

use crate::sync::SyncManager;
use crate::watcher_ignore::IgnoreRules;
use anyhow::{Context, Result};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

/// 文件系统事件类型
pub enum FsEventType {
    /// 文档变更 (ServerMessage 列表)
    DocChange(Vec<crate::protocol::ServerMessage>),
    /// 目录结构变更 (repo-scoped)
    DirChange {
        repo_id: crate::models::RepoId,
        path: String,
    },
}

pub struct Watcher {
    sync_manager: Arc<SyncManager>,
    root_path: std::path::PathBuf,
    ignore_rules: IgnoreRules,
    on_event: Option<Box<dyn Fn(FsEventType) + Send + Sync>>,
}

impl Watcher {
    pub fn new(sync_manager: Arc<SyncManager>, root_path: std::path::PathBuf) -> Self {
        let ignore_rules = IgnoreRules::load(&root_path);
        Self {
            sync_manager,
            root_path,
            ignore_rules,
            on_event: None,
        }
    }

    pub fn with_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn(FsEventType) + Send + Sync + 'static,
    {
        self.on_event = Some(Box::new(cb));
        self
    }

    pub fn watch(&self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        let root_absolute = std::fs::canonicalize(&self.root_path)?;
        let mut debouncer = new_debouncer(Duration::from_millis(300), tx)?;
        debouncer
            .watcher()
            .watch(&root_absolute, RecursiveMode::Recursive)?;

        info!("Watcher started on {:?}", root_absolute);

        for result in rx {
            match result {
                Ok(events) => self.process_events(&events, &root_absolute)?,
                Err(e) => error!("Watch error: {:?}", e),
            }
        }

        Ok(())
    }

    fn process_events(
        &self,
        events: &[notify_debouncer_mini::DebouncedEvent],
        root: &std::path::Path,
    ) -> Result<()> {
        for event in events {
            let path = &event.path;
            if let Ok(rel) = path.strip_prefix(root) {
                let path_str = crate::utils::path::to_forward_slash(&rel.to_string_lossy());

                // 忽略系统目录
                if path_str.starts_with(".git")
                    || crate::utils::notegit::is_internal_repo_path(&path_str)
                {
                    continue;
                }

                // 用户自定义忽略规则 (.deveignore)
                if self.ignore_rules.is_ignored(&path_str) {
                    continue;
                }

                // 目录事件检测 (路径存在且是目录，或不存在但无扩展名)
                let is_dir = classify_dir_event(path)
                    .with_context(|| format!("Failed to classify watcher event for {path_str}"))?;
                if is_dir {
                    if let Some((repo_id, repo_path)) = self
                        .sync_manager
                        .handle_dir_change(&path_str)
                        .with_context(|| format!("Failed to handle dir change for {path_str}"))?
                        && let Some(cb) = &self.on_event
                    {
                        cb(FsEventType::DirChange {
                            repo_id,
                            path: repo_path,
                        });
                    }
                    continue;
                }

                // 只处理 .md 文件
                if !path_str.ends_with(".md") {
                    continue;
                }

                if self.sync_manager.should_ignore_fs_event(&path_str) {
                    continue;
                }

                match self.sync_manager.handle_fs_event(&path_str) {
                    Ok(msgs) if !msgs.is_empty() => {
                        if let Some(cb) = &self.on_event {
                            cb(FsEventType::DocChange(msgs));
                        }
                    }
                    Err(e) => error!("Error handling event for {}: {:?}", path_str, e),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub fn validate_watch_root(root_path: &Path) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let root_absolute = std::fs::canonicalize(root_path)?;
    let mut debouncer = new_debouncer(Duration::from_millis(300), tx)?;
    debouncer
        .watcher()
        .watch(&root_absolute, RecursiveMode::Recursive)?;
    drop(debouncer);
    drop(rx);
    Ok(())
}

fn classify_dir_event(path: &Path) -> Result<bool> {
    match path.try_exists() {
        Ok(false) => Ok(path.extension().is_none()),
        Ok(true) => Ok(std::fs::metadata(path)?.is_dir()),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
#[path = "watcher_test.rs"]
mod tests;
