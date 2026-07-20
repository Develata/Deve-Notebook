//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Session repo binding helpers.

use super::WsSession;
use deve_core::ledger::CatalogMembershipToken;
use deve_core::models::RepoId;

impl WsSession {
    pub fn switch_repo(&mut self, repo_name: String, repo_id: Option<RepoId>) {
        if self.active_repo.as_deref() != Some(&repo_name) || self.active_repo_id != repo_id {
            self.diff_projection_jobs.cancel();
        }
        if self.active_branch.is_none() {
            self.last_local_repo = Some(repo_name.clone());
            self.last_local_repo_id = repo_id;
        }
        self.active_repo = Some(repo_name);
        self.active_repo_id = repo_id;
        self.catalog_membership = None;
    }

    pub fn clear_active_repo(&mut self) {
        self.diff_projection_jobs.cancel();
        self.active_repo = None;
        self.active_repo_id = None;
        self.catalog_membership = None;
    }

    pub(crate) fn bind_catalog_membership(&mut self, token: CatalogMembershipToken) {
        debug_assert_eq!(self.active_repo_id, Some(token.repo_id()));
        self.catalog_membership = Some(token);
    }

    pub(crate) fn catalog_membership_token(&self) -> Option<&CatalogMembershipToken> {
        self.catalog_membership.as_ref()
    }

    pub(crate) fn bind_repo_session_runtime(&mut self, id: u64) {
        self.repo_session_runtime_id = Some(id);
    }

    pub(crate) fn repo_session_runtime_id(&self) -> Option<u64> {
        self.repo_session_runtime_id
    }

    pub(crate) fn commit_no_scope(&mut self, removed_repo_id: RepoId, scope_nonce: u64) {
        self.set_scope_nonce(Some(scope_nonce));
        self.clear_sync_binding();
        self.switch_branch(None);
        self.clear_active_repo();
        self.clear_active_db();
        if self.last_local_repo_id == Some(removed_repo_id) {
            self.last_local_repo = None;
            self.last_local_repo_id = None;
        }
    }
}
