use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::time::Duration;
use tracing::{info,  error};
use std::sync::Arc;
use crate::sync::SyncManager;

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
