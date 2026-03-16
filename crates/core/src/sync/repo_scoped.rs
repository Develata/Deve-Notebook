// crates/core/src/sync/repo_scoped.rs
//! # Repo-Scoped 同步引擎管理器
//!
//! 管理多个仓库的 SyncEngine 实例，每个仓库拥有独立的同步状态。

use crate::models::{PeerId, RepoId};
use crate::security::RepoKey;
use crate::sync::engine::SyncEngine;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Repo-Scoped 同步引擎管理器
///
/// **功能**:
/// - 为每个仓库维护独立的 SyncEngine 实例
/// - 确保不同仓库的同步状态完全隔离 (INV-1)
///
/// ## 不变量 (Invariants)
///
/// - 每个 `RepoId` 对应唯一的 `SyncEngine` 实例
/// - 仓库间不会共享 Version Vector 或 pending ops
/// - 自动按需创建 SyncEngine，无需预先初始化所有仓库
pub struct RepoScopedSyncEngine {
    local_peer_id: PeerId,
    repo: Arc<crate::ledger::RepoManager>,
    sync_mode: crate::config::SyncMode,
    engines: RwLock<HashMap<RepoId, SyncEngine>>,
}

impl RepoScopedSyncEngine {
    fn read_engines(&self) -> RwLockReadGuard<'_, HashMap<RepoId, SyncEngine>> {
        self.engines.read().unwrap_or_else(|e| {
            tracing::warn!("RepoScopedSyncEngine read lock poisoned; recovering");
            e.into_inner()
        })
    }

    fn write_engines(&self) -> RwLockWriteGuard<'_, HashMap<RepoId, SyncEngine>> {
        self.engines.write().unwrap_or_else(|e| {
            tracing::warn!("RepoScopedSyncEngine write lock poisoned; recovering");
            e.into_inner()
        })
    }

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
        }
    }

    /// 获取或创建指定仓库的 SyncEngine
    ///
    /// ## 参数
    /// - `repo_id`: 仓库 ID
    ///
    /// ## 返回值
    /// - `Some(SyncEngine)` - 如果成功获取或创建
    /// - `None` - 如果锁被污染
    pub fn get_or_create(&self, repo_id: RepoId) -> Option<SyncEngine> {
        // 先尝试读取
        let engines = self.read_engines();
        if let Some(engine) = engines.get(&repo_id) {
            return Some(engine.clone());
        }
        drop(engines);

        // 需要创建新的 engine
        let mut engines = self.write_engines();

        if let Some(engine) = engines.get(&repo_id) {
            return Some(engine.clone());
        }

        let engine = SyncEngine::new(
            self.local_peer_id.clone(),
            self.repo.clone(),
            self.sync_mode,
            self.load_repo_key(repo_id),
        );
        engines.insert(repo_id, engine.clone());
        Some(engine)
    }

    /// 严格获取指定仓库的 SyncEngine。
    ///
    /// Invariants:
    /// - repo-scoped sync engine 进入传输链前必须已加载有效 `RepoKey`。
    /// - 严格路径不得缓存 `repo_key = None` 的 engine。
    pub fn get_or_create_strict(&self, repo_id: RepoId) -> Result<SyncEngine> {
        let engines = self
            .engines
            .read()
            .map_err(|_| anyhow!("RepoScopedSyncEngine read lock poisoned"))?;
        if let Some(engine) = engines.get(&repo_id)
            && engine.repo_key.is_some()
        {
            return Ok(engine.clone());
        }
        drop(engines);

        let repo_key = self.load_repo_key_strict(repo_id)?;
        let mut engines = self
            .engines
            .write()
            .map_err(|_| anyhow!("RepoScopedSyncEngine write lock poisoned"))?;

        if let Some(engine) = engines.get_mut(&repo_id) {
            if engine.repo_key.is_none() {
                engine.repo_key = Some(repo_key.clone());
            }
            return Ok(engine.clone());
        }

        let engine = SyncEngine::new(
            self.local_peer_id.clone(),
            self.repo.clone(),
            self.sync_mode,
            Some(repo_key),
        );
        engines.insert(repo_id, engine.clone());
        Ok(engine)
    }

    pub fn get(&self, repo_id: RepoId) -> Option<SyncEngine> {
        let engines = self.read_engines();
        engines.get(&repo_id).cloned()
    }

    /// 对指定仓库的 SyncEngine 执行操作
    pub fn with_engine<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&SyncEngine) -> R,
    {
        let engines = self.read_engines();
        engines.get(&repo_id).map(f)
    }

    /// 对指定仓库的 SyncEngine 执行可变操作
    pub fn with_engine_mut<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&mut SyncEngine) -> R,
    {
        let mut engines = self.write_engines();
        engines.get_mut(&repo_id).map(f)
    }

    /// 移除指定仓库的 SyncEngine
    pub fn remove(&self, repo_id: RepoId) -> Option<SyncEngine> {
        let mut engines = self.write_engines();
        engines.remove(&repo_id)
    }

    /// 获取所有已加载的仓库 ID
    pub fn loaded_repos(&self) -> Vec<RepoId> {
        let engines = match self.engines.read() {
            Ok(e) => e,
            Err(e) => e.into_inner(),
        };
        engines.keys().cloned().collect()
    }

    /// 清空所有 SyncEngine
    pub fn clear(&self) {
        self.write_engines().clear();
    }

    fn load_repo_key(&self, repo_id: RepoId) -> Option<RepoKey> {
        self.load_repo_key_strict(repo_id)
            .map_err(|err| {
                tracing::warn!("RepoScopedSyncEngine failed to load repo key {repo_id}: {err}");
            })
            .ok()
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
        let engines = match self.engines.read() {
            Ok(e) => e.clone(),
            Err(e) => e.into_inner().clone(),
        };

        Self {
            local_peer_id: self.local_peer_id.clone(),
            repo: self.repo.clone(),
            sync_mode: self.sync_mode,
            engines: RwLock::new(engines),
        }
    }
}

#[cfg(test)]
#[path = "repo_scoped_test.rs"]
mod tests;
