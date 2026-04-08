//! plan_ref:
//!   - 04_storage.md §Watcher Architecture

pub use crate::sync::watcher::{WatcherError, start_repo_watcher, stop_repo_watcher};

pub fn validate_watch_root(root_path: &std::path::Path) -> anyhow::Result<()> {
    std::fs::canonicalize(root_path)?;
    Ok(())
}
