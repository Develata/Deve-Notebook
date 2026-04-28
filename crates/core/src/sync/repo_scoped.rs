// crates/core/src/sync/repo_scoped.rs
//! # Repo-Scoped 同步引擎管理器
//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
//! 管理多个仓库的 SyncEngine 实例，每个仓库拥有独立的同步状态。
#[path = "repo_scoped_hydration.rs"]
mod hydration;
#[path = "repo_scoped_lock.rs"]
mod lock;

use crate::models::{PeerId, RepoId};
use crate::security::RepoKey;
use crate::sync::engine::SyncEngine;
use anyhow::{Result, anyhow};
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

    /// 严格获取指定仓库的 SyncEngine。
    ///
    /// Invariants:
    /// - repo-scoped sync engine 进入传输链前必须已加载有效 `RepoKey`。
    /// - 严格路径不得缓存 `repo_key = None` 的 engine。
    pub fn get_or_create_strict(&self, repo_id: RepoId) -> Result<SyncEngine> {
        self.ensure_strict_engine_loaded(repo_id)?;
        self.refresh_loaded_engine_vector(repo_id)?;
        let engines = self.read_engines_result()?;
        let engine = engines
            .get(&repo_id)
            .ok_or_else(|| anyhow!("RepoScopedSyncEngine missing loaded repo {}", repo_id))?;
        if engine.repo_key.is_none() {
            return Err(anyhow!("RepoScopedSyncEngine missing repo key {}", repo_id));
        }
        Ok(engine.clone())
    }

    pub fn with_strict_engine<F, R>(&self, repo_id: RepoId, f: F) -> Result<R>
    where
        F: FnOnce(&SyncEngine) -> R,
    {
        self.ensure_strict_engine_loaded(repo_id)?;
        self.refresh_loaded_engine_vector(repo_id)?;
        let engines = self.read_engines_result()?;
        let engine = engines
            .get(&repo_id)
            .ok_or_else(|| anyhow!("RepoScopedSyncEngine missing loaded repo {}", repo_id))?;
        if engine.repo_key.is_none() {
            return Err(anyhow!("RepoScopedSyncEngine missing repo key {}", repo_id));
        }
        Ok(f(engine))
    }

    pub fn with_strict_engine_mut<F, R>(&self, repo_id: RepoId, f: F) -> Result<R>
    where
        F: FnOnce(&mut SyncEngine) -> R,
    {
        self.ensure_strict_engine_loaded(repo_id)?;
        self.refresh_loaded_engine_vector(repo_id)?;
        let mut engines = self.write_engines_result()?;
        let engine = engines
            .get_mut(&repo_id)
            .ok_or_else(|| anyhow!("RepoScopedSyncEngine missing loaded repo {}", repo_id))?;
        if engine.repo_key.is_none() {
            return Err(anyhow!("RepoScopedSyncEngine missing repo key {}", repo_id));
        }
        Ok(f(engine))
    }

    fn ensure_strict_engine_loaded(&self, repo_id: RepoId) -> Result<()> {
        let engines = self.read_engines_result()?;
        if let Some(engine) = engines.get(&repo_id)
            && engine.repo_key.is_some()
        {
            return Ok(());
        }
        drop(engines);
        let repo_key = self.load_repo_key_strict(repo_id)?;
        let mut engines = self.write_engines_result()?;

        if let Some(engine) = engines.get_mut(&repo_id) {
            if engine.repo_key.is_none() {
                engine.repo_key = Some(repo_key);
            }
            return Ok(());
        }

        let engine = SyncEngine::new(
            self.local_peer_id.clone(),
            self.repo.clone(),
            self.sync_mode,
            Some(repo_key),
        );
        engines.insert(repo_id, engine);
        Ok(())
    }

    fn refresh_loaded_engine_vector(&self, repo_id: RepoId) -> Result<()> {
        let mut engines = self.write_engines_result()?;
        {
            let engine = engines
                .get(&repo_id)
                .ok_or_else(|| anyhow!("RepoScopedSyncEngine missing loaded repo {}", repo_id))?;
            if engine.repo_key.is_none() {
                return Err(anyhow!("RepoScopedSyncEngine missing repo key {}", repo_id));
            }
        }
        let vector =
            hydration::build_version_vector(self.repo.as_ref(), &self.local_peer_id, repo_id)?;
        let engine = engines
            .get_mut(&repo_id)
            .ok_or_else(|| anyhow!("RepoScopedSyncEngine missing loaded repo {}", repo_id))?;
        engine.version_vector = vector;
        Ok(())
    }

    pub fn get(&self, repo_id: RepoId) -> Option<SyncEngine> {
        let engines = self.read_engines()?;
        engines.get(&repo_id).cloned()
    }

    /// 对指定仓库的 SyncEngine 执行操作
    pub fn with_engine<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&SyncEngine) -> R,
    {
        let engines = self.read_engines()?;
        engines.get(&repo_id).map(f)
    }

    /// 对指定仓库的 SyncEngine 执行可变操作
    pub fn with_engine_mut<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&mut SyncEngine) -> R,
    {
        let mut engines = self.write_engines()?;
        engines.get_mut(&repo_id).map(f)
    }

    /// 移除指定仓库的 SyncEngine
    pub fn remove(&self, repo_id: RepoId) -> Option<SyncEngine> {
        let mut engines = self.write_engines()?;
        engines.remove(&repo_id)
    }

    /// 获取所有已加载的仓库 ID
    pub fn loaded_repos(&self) -> Result<Vec<RepoId>> {
        let engines = self
            .read_engines()
            .ok_or_else(|| anyhow!("RepoScopedSyncEngine registry poisoned"))?;
        Ok(engines.keys().cloned().collect())
    }

    /// 清空所有 SyncEngine
    pub fn clear(&self) -> Result<()> {
        let mut engines = self
            .write_engines()
            .ok_or_else(|| anyhow!("RepoScopedSyncEngine registry poisoned"))?;
        engines.clear();
        Ok(())
    }

    fn load_repo_key_strict(&self, repo_id: RepoId) -> Result<RepoKey> {
        let repo_name = match self.repo.find_local_repo_name_by_id(repo_id) {
            Ok(Some(repo_name)) => repo_name,
            Ok(None) => {
                return Err(anyhow!("Local repo not found for UUID {}", repo_id));
            }
            Err(err) => {
                return Err(
                    err.context(format!("Failed to resolve local repo for UUID {}", repo_id))
                );
            }
        };
        let key_dir = match self.repo.local_repo_notegit_keys_root(&repo_name) {
            Ok(key_dir) => key_dir,
            Err(err) => {
                return Err(err.context(format!(
                    "Failed to resolve key dir for local repo {}",
                    repo_name
                )));
            }
        };
        match crate::security::load_or_generate_repo_key_at(&key_dir) {
            Ok(key) => Ok(key),
            Err(err) => Err(err.context(format!(
                "Failed to load repo key for local repo {}",
                repo_name
            ))),
        }
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
#[path = "repo_scoped_test.rs"]
mod tests;
