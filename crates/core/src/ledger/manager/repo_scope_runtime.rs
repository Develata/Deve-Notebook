//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::ledger::manager::{
    repo_catalog_entries::redb_repo_entries,
    repo_selector_resolution::{LocalRepoCandidates, select_repo_name},
};
use crate::ledger::traits::RepoSelector;
use crate::models::RepoId;
use anyhow::Result;

pub(crate) struct RepoScopeRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> RepoScopeRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn resolve_local_selector_for_execution(
        &self,
        repo: &RepoSelector,
    ) -> Result<String> {
        self.resolve_local_repo_name_for_execution(repo.repo_id, repo.repo_name.as_deref())
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
        if let Some(info) =
            RepoManager::read_repo_info_from_db(&self.manager.local_db).map_err(|err| {
                anyhow::anyhow!(
                    "Broken local repo {} while resolving UUID {} without repair: {}",
                    self.manager.local_repo_name,
                    target_id,
                    err
                )
            })?
            && info.uuid == target_id
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
            let repo_uuid = Some(
                RepoManager::read_required_repo_info_from_path(
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
                })?
                .uuid,
            );
            if repo_uuid == Some(target_id) {
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
            return RepoManager::read_repo_info_from_db(&self.manager.local_db);
        }
        self.manager
            .run_on_local_repo_stem(stem, RepoManager::read_repo_info_from_db)
    }

    pub(crate) fn resolve_local_repo_name(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        let initial = self.resolve_local_repo_candidates_with_repair(repo_id, repo_name)?;
        self.select_local_repo_name(&initial)
    }

    pub(crate) fn resolve_local_repo_name_for_execution(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        let candidates = self.resolve_local_repo_candidates_with_repair(repo_id, repo_name)?;
        self.select_local_repo_name_for_execution(&candidates)
    }

    fn resolve_local_repo_candidates(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<LocalRepoCandidates> {
        let by_id = match repo_id {
            Some(repo_id) => Some(
                self.find_local_repo_name_by_id(repo_id)?
                    .ok_or_else(|| anyhow::anyhow!("Local repo not found for UUID {}", repo_id))?,
            ),
            None => None,
        };
        let by_name = match repo_name {
            Some(repo_name) => {
                let normalized = repo_name.trim_end_matches(".redb");
                if let Some(stem) = self.manager.resolve_local_repo_stem(normalized)? {
                    self.manager
                        .get_repo_info_for(None, Some(&stem))?
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Broken local repo {} while resolving selector {}: repository metadata missing",
                                stem,
                                normalized
                            )
                        })?;
                    Some(stem)
                } else {
                    anyhow::bail!("Local repo not found for name {}", normalized);
                }
            }
            None => None,
        };
        Ok(LocalRepoCandidates { by_id, by_name })
    }

    fn select_local_repo_name(&self, candidates: &LocalRepoCandidates) -> Result<String> {
        select_repo_name(candidates, || {
            self.manager
                .repo_catalog_runtime()
                .list_local_display_names()
        })
    }

    fn select_local_repo_name_for_execution(
        &self,
        candidates: &LocalRepoCandidates,
    ) -> Result<String> {
        select_repo_name(candidates, || {
            self.manager
                .repo_catalog_runtime()
                .list_local_execution_names()
        })
    }

    fn resolve_local_repo_candidates_with_repair(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<LocalRepoCandidates> {
        let initial = self.resolve_local_repo_candidates(repo_id, repo_name)?;
        if self.select_local_repo_name(&initial).is_ok() {
            return Ok(initial);
        }
        self.manager
            .repo_catalog_runtime()
            .refresh_local_catalog()?;
        self.resolve_local_repo_candidates(repo_id, repo_name)
    }
}

impl RepoManager {
    pub(crate) fn repo_scope_runtime(&self) -> RepoScopeRuntime<'_> {
        RepoScopeRuntime::new(self)
    }
}
