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
use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

/// 文件系统事件类型
pub enum FsEventType {
    /// 文档变更 (ServerMessage 列表)
    DocChange(Vec<crate::protocol::ServerMessage>),
    /// 目录结构变更 (需要重新扫描树)
    DirChange,
}

pub struct Watcher {
    sync_manager: Arc<SyncManager>,
    root_path: std::path::PathBuf,
    on_event: Option<Box<dyn Fn(FsEventType) + Send + Sync>>,
}

impl Watcher {
    pub fn new(sync_manager: Arc<SyncManager>, root_path: std::path::PathBuf) -> Self {
        Self {
            sync_manager,
            root_path,
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
                Ok(events) => self.process_events(&events, &root_absolute),
                Err(e) => error!("Watch error: {:?}", e),
            }
        }

        Ok(())
    }

    fn process_events(
        &self,
        events: &[notify_debouncer_mini::DebouncedEvent],
        root: &std::path::Path,
    ) {
        let mut dir_changed = false;

        for event in events {
            let path = &event.path;
            if let Ok(rel) = path.strip_prefix(root) {
                let path_str = crate::utils::path::to_forward_slash(&rel.to_string_lossy());

                // 忽略系统目录
                if path_str.starts_with(".git") || path_str.starts_with(".deve") {
                    continue;
                }

                // 目录事件检测 (路径存在且是目录，或不存在但无扩展名)
                let is_dir = path.is_dir() || (!path.exists() && path.extension().is_none());
                if is_dir {
                    dir_changed = true;
                    continue;
                }

                // 只处理 .md 文件
                if !path_str.ends_with(".md") {
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

        // 目录结构变更，通知重新扫描
        if dir_changed && let Some(cb) = &self.on_event {
            cb(FsEventType::DirChange);
        }
    }
}
