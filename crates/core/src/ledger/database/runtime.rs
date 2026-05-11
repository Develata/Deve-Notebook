//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 06_repository#repo-selector-resolution-contract

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
        let name = repo_name.trim_end_matches(".redb");

        match branch {
            None => self.open_local_database(name),
            Some(peer_id) => self.open_remote_database(peer_id, name),
        }
    }

    fn open_local_database(&self, name: &str) -> Result<DatabaseHandle> {
        let stem = self
            .manager
            .resolve_local_repo_name_for_execution(None, Some(name))?;
        let db = self.manager.get_or_open_local_db(&stem)?;
        let repo_id = RepoManager::read_repo_info_from_db(db.as_ref())?
            .map(|info| info.uuid)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Broken local repo {} while opening database: repository metadata missing",
                    stem
                )
            })?;
        Ok(DatabaseHandle {
            db,
            readonly: false,
            branch: None,
            repo_id: Some(repo_id),
            repo_name: stem,
        })
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
            return Ok(DatabaseHandle {
                db,
                readonly: true,
                branch: Some(peer_id.clone()),
                repo_id: Some(repo_id),
                repo_name,
            });
        }
        let db = self.manager.get_or_open_db_at(&resolved.path)?;
        Ok(DatabaseHandle {
            db,
            readonly: true,
            branch: Some(peer_id.clone()),
            repo_id: Some(repo_id),
            repo_name,
        })
    }
}
