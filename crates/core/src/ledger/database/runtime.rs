//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#repo-selector-resolution-contract

use super::DatabaseHandle;
use crate::ledger::RepoManager;
use crate::models::PeerId;
use anyhow::Result;

pub(crate) struct RepoDatabaseRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> RepoDatabaseRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn open_database(
        &self,
        branch: Option<&PeerId>,
        repo_name: &str,
    ) -> Result<DatabaseHandle> {
        match branch {
            None => self.open_local_database(repo_name),
            Some(peer_id) => self.open_remote_database(peer_id, repo_name),
        }
    }

    fn open_local_database(&self, name: &str) -> Result<DatabaseHandle> {
        let stem = self
            .manager
            .resolve_local_repo_name_for_execution(None, Some(name))?;
        let lease = self.manager.lease_local_authority_stem(&stem)?;
        let repo_id = RepoManager::read_local_repo_info_from_db(lease.db())?
            .map(|info| info.uuid)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Broken local repo {} while opening database: repository metadata missing",
                    stem
                )
            })?;
        Ok(DatabaseHandle::local(repo_id, stem))
    }

    fn open_remote_database(&self, peer_id: &PeerId, name: &str) -> Result<DatabaseHandle> {
        let resolved = self
            .manager
            .repo_catalog_runtime()
            .resolve_remote_repo_entry(peer_id, name)?
            .ok_or_else(|| anyhow::anyhow!("Repository not found: {}", name))?;
        let repo_name = resolved.stem.clone();
        let repo_id = match &resolved.info {
            Some(info) => info.uuid,
            None => uuid::Uuid::parse_str(&resolved.stem).map_err(|_| {
                anyhow::anyhow!(
                    "Broken remote repo {} for peer {} while opening database: repository metadata missing",
                    resolved.stem,
                    peer_id
                )
            })?,
        };
        let loaded = self
            .manager
            .read_shadow_dbs()?
            .get(peer_id)
            .and_then(|repos| repos.get(&repo_id))
            .cloned();
        if let Some(db) = loaded {
            return Ok(DatabaseHandle::remote(
                db,
                peer_id.clone(),
                repo_id,
                repo_name,
            ));
        }
        let db = self.manager.get_or_open_db_at(&resolved.path)?;
        Ok(DatabaseHandle::remote(
            db,
            peer_id.clone(),
            repo_id,
            repo_name,
        ))
    }
}
