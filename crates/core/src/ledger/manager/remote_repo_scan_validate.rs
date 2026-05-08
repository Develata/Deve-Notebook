//! plan_ref:
//!   - 06_repository#repo-catalog-contract

use crate::ledger::manager::remote_repo_scan_entry::RemoteRepoEntry;
use crate::ledger::manager::remote_repo_scan_helpers::resolve_remote_repo_entry_by_id;
use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use anyhow::Result;

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

impl RepoManager {
    pub(crate) fn resolve_remote_repo_entry_by_id(
        &self,
        peer_id: &PeerId,
        repo_id: uuid::Uuid,
    ) -> Result<Option<RemoteRepoEntry>> {
        resolve_remote_repo_entry_by_id(self, peer_id, repo_id)
    }
}
