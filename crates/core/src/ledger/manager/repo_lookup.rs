//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 06_repository#repo-selector-resolution-contract

use anyhow::Result;

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;

impl RepoManager {
    pub fn get_local_repo_info_by_id(&self, repo_id: uuid::Uuid) -> Result<Option<RepoInfo>> {
        self.repo_scope_runtime().get_local_repo_info_by_id(repo_id)
    }

    pub fn get_repo_url(&self, branch: Option<&PeerId>, repo_name: &str) -> Result<Option<String>> {
        Ok(self
            .get_repo_info_for(branch, Some(repo_name))?
            .and_then(|info| info.url))
    }

    pub fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        self.repo_scope_runtime()
            .find_local_repo_name_by_url(target_url)
    }

    pub fn get_repo_info_for(
        &self,
        branch: Option<&PeerId>,
        repo_name: Option<&str>,
    ) -> Result<Option<RepoInfo>> {
        let name = repo_name
            .unwrap_or(&self.local_repo_name)
            .trim_end_matches(".redb");
        if let Some(peer_id) = branch {
            return self.read_remote_repo_info(peer_id, name);
        }
        if let Some(stem) = self.resolve_local_repo_stem(name)? {
            return self
                .repo_scope_runtime()
                .read_local_repo_info_by_stem_without_repair(&stem);
        }
        self.refresh_local_repo_catalog()?;
        if let Some(stem) = self.resolve_local_repo_stem(name)? {
            return self
                .repo_scope_runtime()
                .read_local_repo_info_by_stem_without_repair(&stem);
        }
        Ok(None)
    }

    fn read_remote_repo_info(&self, peer_id: &PeerId, repo_name: &str) -> Result<Option<RepoInfo>> {
        if let Some(entry) = self.resolve_remote_repo_entry(peer_id, repo_name)? {
            if let Some(info) = entry.info {
                return Ok(Some(info));
            }
            return Ok(None);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests;
