//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::RepoId;
use anyhow::Result;

pub(crate) struct RepoScopeLookupRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> RepoScopeLookupRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn find_local_repo_name_by_id(&self, target_id: RepoId) -> Result<Option<String>> {
        self.manager
            .repo_catalog_runtime()
            .refresh_local_catalog()?;
        self.find_local_repo_name_by_id_without_repair(target_id)
    }

    pub(crate) fn find_local_repo_name_by_id_without_repair(
        &self,
        target_id: RepoId,
    ) -> Result<Option<String>> {
        if !self.manager.normal_repo_catalog_ids()?.contains(&target_id) {
            return Ok(None);
        }
        let stem = target_id.to_string();
        let lease = self
            .manager
            .lease_local_authority(target_id)
            .map_err(|err| {
                anyhow::anyhow!(
                    "Broken local repo {} while resolving UUID {} without repair: {}",
                    stem,
                    target_id,
                    err
                )
            })?;
        let info = RepoManager::read_local_repo_info_from_db(lease.db())?.ok_or_else(|| {
            anyhow::anyhow!(
                "Broken local repo {} while resolving UUID without repair: repository metadata missing",
                stem
            )
        })?;
        if info.uuid != target_id || info.name != stem {
            anyhow::bail!(
                "Broken local repo {} while resolving UUID without repair: metadata identity mismatch",
                stem
            );
        }
        Ok(Some(stem))
    }

    pub(crate) fn get_local_repo_info_by_id(&self, repo_id: RepoId) -> Result<Option<RepoInfo>> {
        self.manager
            .repo_catalog_runtime()
            .refresh_local_catalog()?;
        self.get_local_repo_info_by_id_without_repair(repo_id)
    }

    pub(crate) fn get_local_repo_info_by_id_without_repair(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<RepoInfo>> {
        let Some(repo_stem) = self.find_local_repo_name_by_id_without_repair(repo_id)? else {
            return Ok(None);
        };
        self.read_local_repo_info_by_stem_without_repair(&repo_stem)
    }

    pub(crate) fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        self.manager
            .repo_catalog_runtime()
            .refresh_local_catalog()?;
        let mut matches = Vec::new();
        for repo_name in self
            .manager
            .repo_catalog_runtime()
            .list_local_execution_names()?
        {
            let info = self
                .read_local_repo_info_by_stem_without_repair(&repo_name)?
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

    pub(crate) fn read_local_repo_info_by_stem_without_repair(
        &self,
        stem: &str,
    ) -> Result<Option<RepoInfo>> {
        let lease = self.manager.lease_local_authority_stem(stem)?;
        RepoManager::read_local_repo_info_from_db(lease.db())
    }
}
