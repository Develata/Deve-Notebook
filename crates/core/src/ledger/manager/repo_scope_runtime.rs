//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::manager::types::RepoManager;
use crate::ledger::traits::RepoSelector;
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
        self.manager
            .resolve_local_repo_name_for_execution(repo.repo_id, repo.repo_name.as_deref())
    }
}

impl RepoManager {
    pub(crate) fn repo_scope_runtime(&self) -> RepoScopeRuntime<'_> {
        RepoScopeRuntime::new(self)
    }
}
