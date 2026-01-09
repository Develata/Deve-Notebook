use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::time::Duration;
use tracing::{info,  error};
use std::sync::Arc;
use crate::sync::SyncManager;

pub struct Watcher {
    sync_manager: Arc<SyncManager>,
    root_path: std::path::PathBuf,
}

impl Watcher {
    pub fn new(sync_manager: Arc<SyncManager>, root_path: std::path::PathBuf) -> Self {
        Self { sync_manager, root_path }
    }

    pub fn watch(&self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();

        // 200ms debounce
        let mut debouncer = new_debouncer(Duration::from_millis(200), tx)?;

        debouncer
            .watcher()
            .watch(&self.root_path, RecursiveMode::Recursive)?;

        info!("Watcher started on {:?}", self.root_path);

        for result in rx {
            match result {
                Ok(events) => {
                    for event in events {
                       let path = event.path;
                       // Convert to relative string
                       if let Ok(rel) = path.strip_prefix(&self.root_path) {
                           let path_str = rel.to_string_lossy().to_string();
                           if let Err(e) = self.sync_manager.handle_fs_event(&path_str) {
                               error!("Error handling event for {}: {:?}", path_str, e);
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
