//! plan_ref:
//!   - 04_storage.md §8. Watcher Contract

use super::WatcherError;
use crate::models::RepoId;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::thread::JoinHandle;

pub(crate) struct WatcherHandle {
    pub stop_tx: std::sync::mpsc::Sender<()>,
    pub join: JoinHandle<Result<(), WatcherError>>,
}

static REGISTRY: LazyLock<Mutex<HashMap<RepoId, WatcherHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn insert(repo_id: RepoId, handle: WatcherHandle) -> Result<(), WatcherError> {
    let mut guard = REGISTRY
        .lock()
        .map_err(|_| WatcherError::RegistryPoisoned)?;
    if guard.contains_key(&repo_id) {
        return Err(WatcherError::AlreadyRunning(repo_id));
    }
    guard.insert(repo_id, handle);
    Ok(())
}

pub(crate) fn remove(repo_id: RepoId) -> Result<Option<WatcherHandle>, WatcherError> {
    let mut guard = REGISTRY
        .lock()
        .map_err(|_| WatcherError::RegistryPoisoned)?;
    Ok(guard.remove(&repo_id))
}
