//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime

use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
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
        if let Some(info) = RepoManager::read_local_repo_info_from_db(&self.manager.local_db)
            .map_err(|err| {
                anyhow::anyhow!(
                    "Broken local repo {} while resolving UUID {} without repair: {}",
                    self.manager.local_repo_name,
                    target_id,
                    err
                )
            })?
            && info.uuid == target_id
            && !self.manager.is_local_repo_removed(info.uuid)?
        {
            return Ok(Some(self.manager.local_repo_name.clone()));
        }

        let local_dir = RepoManager::checked_local_dir_for(
            &self.manager.ledger_dir,
            "resolving local repo UUID without repair",
        )?;

        for (path, file_stem) in
            redb_repo_entries(&local_dir, "resolving local repo UUID without repair")?
        {
            if file_stem == self.manager.local_repo_name {
                continue;
            }
            let info = RepoManager::read_required_local_repo_info_from_path(
                &path,
                &file_stem,
                "resolving UUID without repair",
            )
            .map_err(|err| {
                anyhow::anyhow!(
                    "Broken local repo {} while resolving UUID {} without repair: {}",
                    file_stem,
                    target_id,
                    err
                )
            })?;
            if self.manager.is_local_repo_removed(info.uuid)? {
                continue;
            }
            if info.uuid == target_id {
                return Ok(Some(file_stem));
            }
        }

        Ok(None)
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
        if stem == self.manager.local_repo_name {
            return RepoManager::read_local_repo_info_from_db(&self.manager.local_db);
        }
        self.manager
            .run_on_local_repo_stem(stem, RepoManager::read_repo_info_from_db)
    }
}
