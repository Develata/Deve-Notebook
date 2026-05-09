//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::RepoScopedSyncEngine;
use crate::models::RepoId;
use crate::sync::engine::SyncEngine;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

impl RepoScopedSyncEngine {
    pub(super) fn registry_poisoned(&self) -> bool {
        self.poisoned.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(super) fn read_engines_result(
        &self,
    ) -> Result<RwLockReadGuard<'_, HashMap<RepoId, SyncEngine>>> {
        if self.registry_poisoned() {
            return Err(anyhow!("RepoScopedSyncEngine registry poisoned"));
        }
        match self.engines.read() {
            Ok(guard) => Ok(guard),
            Err(_) => {
                self.poisoned
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                Err(anyhow!("RepoScopedSyncEngine read lock poisoned"))
            }
        }
    }

    pub(super) fn write_engines_result(
        &self,
    ) -> Result<RwLockWriteGuard<'_, HashMap<RepoId, SyncEngine>>> {
        if self.registry_poisoned() {
            return Err(anyhow!("RepoScopedSyncEngine registry poisoned"));
        }
        match self.engines.write() {
            Ok(guard) => Ok(guard),
            Err(_) => {
                self.poisoned
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                Err(anyhow!("RepoScopedSyncEngine write lock poisoned"))
            }
        }
    }

    pub(super) fn read_engines(&self) -> Option<RwLockReadGuard<'_, HashMap<RepoId, SyncEngine>>> {
        match self.read_engines_result() {
            Ok(guard) => Some(guard),
            Err(err) => {
                tracing::error!("{}; failing closed", err);
                None
            }
        }
    }

    pub(super) fn write_engines(
        &self,
    ) -> Option<RwLockWriteGuard<'_, HashMap<RepoId, SyncEngine>>> {
        match self.write_engines_result() {
            Ok(guard) => Some(guard),
            Err(err) => {
                tracing::error!("{}; failing closed", err);
                None
            }
        }
    }
}
