//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Session repo binding helpers.

use super::WsSession;
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
    }

    pub fn clear_active_repo(&mut self) {
        self.diff_projection_jobs.cancel();
        self.active_repo = None;
        self.active_repo_id = None;
    }
}
