// crates/core/src/sync/repo_scoped/registry.rs
//! # Repo-Scoped SyncEngine Registry
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Non-strict registry facade for already loaded repo-scoped sync engines.

use super::RepoScopedSyncEngine;
use crate::models::RepoId;
use crate::sync::engine::SyncEngine;
use anyhow::{Result, anyhow};

impl RepoScopedSyncEngine {
    pub fn get(&self, repo_id: RepoId) -> Option<SyncEngine> {
        let engines = self.read_engines()?;
        engines.get(&repo_id).cloned()
    }

    /// 对指定仓库的 SyncEngine 执行操作。
    pub fn with_engine<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&SyncEngine) -> R,
    {
        let engines = self.read_engines()?;
        engines.get(&repo_id).map(f)
    }

    /// 对指定仓库的 SyncEngine 执行可变操作。
    pub fn with_engine_mut<F, R>(&self, repo_id: RepoId, f: F) -> Option<R>
    where
        F: FnOnce(&mut SyncEngine) -> R,
    {
        let mut engines = self.write_engines()?;
        engines.get_mut(&repo_id).map(f)
    }

    /// 移除指定仓库的 SyncEngine。
    pub fn remove(&self, repo_id: RepoId) -> Option<SyncEngine> {
        let mut engines = self.write_engines()?;
        engines.remove(&repo_id)
    }

    /// 获取所有已加载的仓库 ID。
    pub fn loaded_repos(&self) -> Result<Vec<RepoId>> {
        let engines = self
            .read_engines()
            .ok_or_else(|| anyhow!("RepoScopedSyncEngine registry poisoned"))?;
        Ok(engines.keys().cloned().collect())
    }

    /// 清空所有 SyncEngine。
    pub fn clear(&self) -> Result<()> {
        let mut engines = self
            .write_engines()
            .ok_or_else(|| anyhow!("RepoScopedSyncEngine registry poisoned"))?;
        engines.clear();
        Ok(())
    }
}
