//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 06_repository#repo-selector-resolution-contract

use anyhow::Result;

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;

impl RepoManager {
    pub fn get_local_repo_info_by_id(&self, repo_id: uuid::Uuid) -> Result<Option<RepoInfo>> {
        self.refresh_local_repo_catalog()?;
        self.get_local_repo_info_by_id_without_repair(repo_id)
    }

    pub(crate) fn get_local_repo_info_by_id_without_repair(
        &self,
        repo_id: uuid::Uuid,
    ) -> Result<Option<RepoInfo>> {
        let Some(repo_stem) = self.find_local_repo_name_by_id_without_repair(repo_id)? else {
            return Ok(None);
        };
        if repo_stem == self.local_repo_name {
            return self.read_local_repo_info_by_stem_without_repair(&repo_stem);
        }
        self.read_local_repo_info_by_stem_without_repair(&repo_stem)
    }

    pub fn get_repo_url(&self, branch: Option<&PeerId>, repo_name: &str) -> Result<Option<String>> {
        Ok(self
            .get_repo_info_for(branch, Some(repo_name))?
            .and_then(|info| info.url))
    }

    pub fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        self.refresh_local_repo_catalog()?;
        let mut matches = Vec::new();
        for repo_name in self.list_local_repo_names_for_execution()? {
            let info = self
                .get_repo_info_for(None, Some(&repo_name))?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Broken local repo {} while resolving URL {}: repository metadata missing",
                        repo_name,
                        target_url
                    )
                })?;
            if info.url.as_deref() == Some(target_url) {
                matches.push(repo_name);
            }
        }
        if matches.len() > 1 {
            return Err(anyhow::anyhow!(
                "Ambiguous local repository selector for URL {}",
                target_url
            ));
        }
        Ok(matches.into_iter().next())
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
            return self.read_local_repo_info_by_stem_without_repair(&stem);
        }
        self.refresh_local_repo_catalog()?;
        if let Some(stem) = self.resolve_local_repo_stem(name)? {
            return self.read_local_repo_info_by_stem_without_repair(&stem);
        }
        Ok(None)
    }

    fn read_local_repo_info_by_stem_without_repair(&self, stem: &str) -> Result<Option<RepoInfo>> {
        if stem == self.local_repo_name {
            return Self::read_repo_info_from_db(&self.local_db);
        }
        self.run_on_local_repo_stem(stem, Self::read_repo_info_from_db)
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
