//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::manager::repo_scope_lookup_runtime::RepoScopeLookupRuntime;
use crate::ledger::manager::repo_scope_selector_runtime::RepoScopeSelectorRuntime;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
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
        self.selector()
            .resolve_local_repo_name_for_execution(repo.repo_id, repo.repo_name.as_deref())
    }

    pub(crate) fn find_local_repo_name_by_id(&self, target_id: RepoId) -> Result<Option<String>> {
        self.lookup().find_local_repo_name_by_id(target_id)
    }

    #[cfg(test)]
    pub(crate) fn find_local_repo_name_by_id_without_repair(
        &self,
        target_id: RepoId,
    ) -> Result<Option<String>> {
        self.lookup()
            .find_local_repo_name_by_id_without_repair(target_id)
    }

    pub(crate) fn get_local_repo_info_by_id(&self, repo_id: RepoId) -> Result<Option<RepoInfo>> {
        self.lookup().get_local_repo_info_by_id(repo_id)
    }

    #[cfg(test)]
    pub(crate) fn get_local_repo_info_by_id_without_repair(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<RepoInfo>> {
        self.lookup()
            .get_local_repo_info_by_id_without_repair(repo_id)
    }

    pub(crate) fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        self.lookup().find_local_repo_name_by_url(target_url)
    }

    pub(crate) fn read_local_repo_info_by_stem_without_repair(
        &self,
        stem: &str,
    ) -> Result<Option<RepoInfo>> {
        self.lookup()
            .read_local_repo_info_by_stem_without_repair(stem)
    }

    pub(crate) fn resolve_local_repo_name(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        self.selector().resolve_local_repo_name(repo_id, repo_name)
    }

    pub(crate) fn resolve_local_repo_name_for_execution(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        self.selector()
            .resolve_local_repo_name_for_execution(repo_id, repo_name)
    }

    fn lookup(&self) -> RepoScopeLookupRuntime<'a> {
        RepoScopeLookupRuntime::new(self.manager)
    }

    fn selector(&self) -> RepoScopeSelectorRuntime<'a> {
        RepoScopeSelectorRuntime::new(self.manager)
    }
}

impl RepoManager {
    pub(crate) fn repo_scope_runtime(&self) -> RepoScopeRuntime<'_> {
        RepoScopeRuntime::new(self)
    }
}
