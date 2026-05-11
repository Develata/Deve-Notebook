//! plan_ref:
//!   - 06_repository#repo-catalog-contract

use crate::ledger::manager::remote_repo_scan_entry::RemoteRepoEntry;
use crate::ledger::manager::remote_repo_scan_helpers::{
    duplicate_entry_ids, reject_duplicate_remote_matches, single_remote_entry,
};
use crate::ledger::manager::repo_catalog_runtime::RepoCatalogRuntime;
use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use anyhow::{Result, anyhow};

pub(super) fn validate_remote_repo_url_coverage(
    repo: &RepoManager,
    peer_id: &PeerId,
) -> Result<()> {
    for entry in repo.scan_remote_repo_entries(peer_id)? {
        if let Some(info) = &entry.info
            && info.url.as_deref().is_none_or(|u| u.trim().is_empty())
        {
            anyhow::bail!(
                "Broken remote repo {} while validating URL coverage: repository URL not resolved",
                entry.stem
            );
        }
    }
    Ok(())
}

impl<'a> RepoCatalogRuntime<'a> {
    pub(crate) fn resolve_remote_repo_entry_by_id(
        &self,
        peer_id: &PeerId,
        repo_id: uuid::Uuid,
    ) -> Result<Option<RemoteRepoEntry>> {
        let entries = self.scan_remote_repo_entries(peer_id)?;
        if let Some(entry) = entries.iter().find(|entry| !entry.is_readable()) {
            return Err(anyhow!(
                "Broken shadow repo {} for peer {} while resolving selector",
                entry.stem,
                peer_id
            ));
        }
        let duplicate_ids = duplicate_entry_ids(&entries);
        let matches = entries
            .into_iter()
            .filter(|entry| entry.info.as_ref().is_some_and(|info| info.uuid == repo_id))
            .collect::<Vec<_>>();
        reject_duplicate_remote_matches(&repo_id.to_string(), &matches, &duplicate_ids)?;
        Ok(single_remote_entry(matches))
    }
}

impl RepoManager {
    pub(crate) fn resolve_remote_repo_entry_by_id(
        &self,
        peer_id: &PeerId,
        repo_id: uuid::Uuid,
    ) -> Result<Option<RemoteRepoEntry>> {
        self.repo_catalog_runtime()
            .resolve_remote_repo_entry_by_id(peer_id, repo_id)
    }
}
