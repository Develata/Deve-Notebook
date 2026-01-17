// crates\core\src
//! # 文件系统监听器
//!
//! 本模块提供 `Watcher` 结构体用于监控文件系统变更。
//!
//! ## 功能
//!
//! - 防抖的文件系统事件（200ms）避免过度处理
//! - 与 `SyncManager` 集成处理变更
//! - 回调支持，用于通过 WebSocket 广播变更
//!
//! 监听器在阻塞线程中运行，并将事件转发给同步管理器。

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::time::Duration;
use tracing::{info,  error};
use std::sync::Arc;
use crate::sync::SyncManager;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 100;

pub struct Watcher {
    sync_manager: Arc<SyncManager>,
    root_path: std::path::PathBuf,
    on_event: Option<Box<dyn Fn(Vec<crate::protocol::ServerMessage>) + Send + Sync>>,
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
    where F: Fn(Vec<crate::protocol::ServerMessage>) + Send + Sync + 'static 
    {
        self.on_event = Some(Box::new(cb));
        self
    }

    pub fn watch(&self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        
        // Canonicalize the root path to ensure we have an absolute path.
        // This fixes issues on Windows where events have absolute paths but root is relative.
        let root_absolute = std::fs::canonicalize(&self.root_path)?;

        // 200ms debounce
        let mut debouncer = new_debouncer(Duration::from_millis(200), tx)?;

        debouncer
            .watcher()
            .watch(&root_absolute, RecursiveMode::Recursive)?;

        info!("Watcher started on {:?}", root_absolute);

        for result in rx {
            match result {
                Ok(events) => {
                    for event in events {
                       let path = event.path;
                       // Convert to relative string and normalize to forward slashes
                       if let Ok(rel) = path.strip_prefix(&root_absolute) {
                           let path_str = crate::utils::path::to_forward_slash(&rel.to_string_lossy());
                           
                           // Ignore system and hidden directories
                           if path_str.starts_with(".git") || path_str.starts_with(".deve") {
                               continue;
                           }
                           
                           // Only process .md files (skip directories and other files)
                           if !path_str.ends_with(".md") {
                               continue;
                           }

                           match self.sync_manager.handle_fs_event(&path_str) {
                               Ok(msgs) => {
                                   if !msgs.is_empty() {
                                       if let Some(cb) = &self.on_event {
                                           cb(msgs);
                                       }
                                   }
                               }
                               Err(e) => {
                                   error!("Error handling event for {}: {:?}", path_str, e);
                               }
                           }
                       }
                    }
                }
                Err(e) => {
                    error!("Watch error: {:?}", e);
                }
            }
        }

        Ok(())
    }
}
