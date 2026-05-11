//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::manager::repo_scope_lookup_runtime::RepoScopeLookupRuntime;
use crate::ledger::manager::repo_selector_resolution::{LocalRepoCandidates, select_repo_name};
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;
use anyhow::Result;

pub(crate) struct RepoScopeSelectorRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> RepoScopeSelectorRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
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
                self.lookup()
                    .find_local_repo_name_by_id(repo_id)?
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

    fn lookup(&self) -> RepoScopeLookupRuntime<'a> {
        RepoScopeLookupRuntime::new(self.manager)
    }
}
