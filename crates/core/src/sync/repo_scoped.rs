// crates/core/src/sync/repo_scoped.rs
//! # Repo-Scoped 同步引擎管理器
//!
//! 管理多个仓库的 SyncEngine 实例，每个仓库拥有独立的同步状态。

use crate::models::{PeerId, RepoId};
use crate::sync::engine::SyncEngine;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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
    repo_key: Option<crate::security::RepoKey>,
    engines: RwLock<HashMap<RepoId, SyncEngine>>,
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
        repo_key: Option<crate::security::RepoKey>,
    ) -> Self {
        Self {
            local_peer_id,
            repo,
            sync_mode,
            repo_key,
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
        if let Ok(engines) = self.engines.read() {
            if let Some(engine) = engines.get(&repo_id) {
                return Some(engine.clone());
            }
        }

        // 需要创建新的 engine
        let mut engines = self.engines.write().ok()?;

        if let Some(engine) = engines.get(&repo_id) {
            return Some(engine.clone());
        }

        let engine = SyncEngine::new(
            self.local_peer_id.clone(),
            self.repo.clone(),
            self.sync_mode,
            self.repo_key.clone(),
        );
        engines.insert(repo_id, engine.clone());
        Some(engine)
    }

    pub fn get(&self, repo_id: RepoId) -> Option<SyncEngine> {
        let engines = self.engines.read().ok()?;
        engines.get(&repo_id).map(|e| e.clone())
    }

    /// 获取或创建指定仓库的 SyncEngine（可变引用）
    pub fn get_or_create_mut<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&mut SyncEngine) -> R,
    {
        let mut engines = self.engines.write().ok()?;

        let engine = engines.entry(repo_id).or_insert_with(|| {
            SyncEngine::new(
                self.local_peer_id.clone(),
                self.repo.clone(),
                self.sync_mode,
                self.repo_key.clone(),
            )
        });

        Some(f(engine))
    }

    /// 对指定仓库的 SyncEngine 执行操作
    pub fn with_engine<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&SyncEngine) -> R,
    {
        let engines = self.engines.read().ok()?;
        engines.get(&repo_id).map(f)
    }

    /// 对指定仓库的 SyncEngine 执行可变操作
    pub fn with_engine_mut<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&mut SyncEngine) -> R,
    {
        let mut engines = self.engines.write().ok()?;
        engines.get_mut(&repo_id).map(f)
    }

    /// 移除指定仓库的 SyncEngine
    pub fn remove(&self, repo_id: RepoId) -> Option<SyncEngine> {
        let mut engines = self.engines.write().ok()?;
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
        if let Ok(mut engines) = self.engines.write() {
            engines.clear();
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
            repo_key: self.repo_key.clone(),
            engines: RwLock::new(engines),
        }
    }
}
