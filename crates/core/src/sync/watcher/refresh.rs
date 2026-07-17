//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!
//! Project-owned refresh notifications emitted after pending-state changes.

use crate::models::RepoId;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatcherRefreshKind {
    Added,
    Modified,
    Deleted,
    DirectoryChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatcherRefresh {
    repo_id: RepoId,
    path: String,
    kind: WatcherRefreshKind,
    has_conflict: bool,
}

impl WatcherRefresh {
    pub fn new(
        repo_id: RepoId,
        path: impl Into<String>,
        kind: WatcherRefreshKind,
        has_conflict: bool,
    ) -> Self {
        Self {
            repo_id,
            path: path.into(),
            kind,
            has_conflict,
        }
    }

    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn kind(&self) -> WatcherRefreshKind {
        self.kind
    }

    pub fn has_conflict(&self) -> bool {
        self.has_conflict
    }
}

pub type WatcherRefreshCallback = Arc<dyn Fn(WatcherRefresh) + Send + Sync>;
