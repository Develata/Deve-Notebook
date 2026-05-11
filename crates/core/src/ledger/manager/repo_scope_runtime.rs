//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::manager::types::RepoManager;
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
