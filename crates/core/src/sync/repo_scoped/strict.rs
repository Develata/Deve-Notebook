// crates/core/src/sync/repo_scoped/strict.rs
//! # Repo-Scoped Strict Engine Loading
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Strict repo-scoped sync engine loading for transport-facing paths.

use super::{RepoScopedSyncEngine, hydration};
use crate::models::RepoId;
use crate::security::RepoKey;
use crate::sync::engine::SyncEngine;
use anyhow::{Result, anyhow};

impl RepoScopedSyncEngine {
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
