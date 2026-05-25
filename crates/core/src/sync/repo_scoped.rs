// crates/core/src/sync/repo_scoped.rs
//! # Repo-Scoped 同步引擎管理器
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! 管理多个仓库的 SyncEngine 实例，每个仓库拥有独立的同步状态。
mod hydration;
mod lock;
mod registry;
mod strict;

use crate::models::{PeerId, RepoId};
use crate::sync::engine::SyncEngine;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// Repo-Scoped 同步引擎管理器。
///
/// 不变量:
/// - 每个 `RepoId` 对应唯一的 `SyncEngine`
/// - 仓库间不同步状态完全隔离
/// - registry 一旦锁污染，后续必须 fail-closed
pub struct RepoScopedSyncEngine {
    local_peer_id: PeerId,
    repo: Arc<crate::ledger::RepoManager>,
    sync_mode: crate::config::SyncMode,
    engines: RwLock<HashMap<RepoId, SyncEngine>>,
    poisoned: AtomicBool,
}
impl RepoScopedSyncEngine {
    /// 创建新的 Repo-Scoped 同步引擎管理器
    ///
    /// ## 后置条件 (Post-conditions)
    /// - `engines` 为空 HashMap
    /// - 首次访问某个 repo 时才会创建对应的 SyncEngine
    pub fn new(
        local_peer_id: PeerId,
        repo: Arc<crate::ledger::RepoManager>,
        sync_mode: crate::config::SyncMode,
    ) -> Self {
        Self {
            local_peer_id,
            repo,
            sync_mode,
            engines: RwLock::new(HashMap::new()),
            poisoned: AtomicBool::new(false),
        }
    }

    pub fn sync_mode(&self) -> crate::config::SyncMode {
        self.sync_mode
    }
}

impl Clone for RepoScopedSyncEngine {
    fn clone(&self) -> Self {
        let mut poisoned = self.registry_poisoned();
        let engines = if poisoned {
            tracing::error!(
                "RepoScopedSyncEngine clone observed poisoned engine registry; cloning closed"
            );
            HashMap::new()
        } else {
            match self.engines.read() {
                Ok(engines) => engines.clone(),
                Err(_) => {
                    self.poisoned.store(true, Ordering::Relaxed);
                    poisoned = true;
                    tracing::error!(
                        "RepoScopedSyncEngine clone observed poisoned engine registry; cloning closed"
                    );
                    HashMap::new()
                }
            }
        };

        Self {
            local_peer_id: self.local_peer_id.clone(),
            repo: self.repo.clone(),
            sync_mode: self.sync_mode,
            engines: RwLock::new(engines),
            poisoned: AtomicBool::new(poisoned),
        }
    }
}
#[cfg(test)]
mod tests;
