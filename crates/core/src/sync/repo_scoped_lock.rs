use super::RepoScopedSyncEngine;
use crate::models::RepoId;
use crate::sync::engine::SyncEngine;
use std::collections::HashMap;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

impl RepoScopedSyncEngine {
    pub(super) fn registry_poisoned(&self) -> bool {
        self.poisoned.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(super) fn read_engines(
        &self,
    ) -> Option<RwLockReadGuard<'_, HashMap<RepoId, SyncEngine>>> {
        if self.registry_poisoned() {
            tracing::error!("RepoScopedSyncEngine registry poisoned; failing closed");
            return None;
        }
        match self.engines.read() {
            Ok(guard) => Some(guard),
            Err(_) => {
                self.poisoned
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                tracing::error!("RepoScopedSyncEngine read lock poisoned; failing closed");
                None
            }
        }
    }

    pub(super) fn write_engines(
        &self,
    ) -> Option<RwLockWriteGuard<'_, HashMap<RepoId, SyncEngine>>> {
        if self.registry_poisoned() {
            tracing::error!("RepoScopedSyncEngine registry poisoned; failing closed");
            return None;
        }
        match self.engines.write() {
            Ok(guard) => Some(guard),
            Err(_) => {
                self.poisoned
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                tracing::error!("RepoScopedSyncEngine write lock poisoned; failing closed");
                None
            }
        }
    }
}
