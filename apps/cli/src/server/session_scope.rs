use super::{WriterIdentity, WsSession};
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::{PeerId, RepoId};

impl WsSession {
    pub fn set_authenticated(&mut self, peer_id: PeerId) {
        if self.authenticated_peer_id.as_ref() != Some(&peer_id) {
            self.writer_identity = None;
        }
        self.authenticated_peer_id = Some(peer_id);
    }

    pub fn mark_browser_session(&mut self) {
        self.browser_session = true;
    }

    pub fn is_browser_session(&self) -> bool {
        self.browser_session
    }

    pub fn bind_repo(&mut self, repo_id: RepoId) {
        if self.bound_repo_id != Some(repo_id) {
            self.writer_identity = None;
        }
        self.bound_repo_id = Some(repo_id);
    }

    pub fn set_writer_identity(&mut self, repo_id: RepoId, peer_id: PeerId) {
        self.writer_identity = Some(WriterIdentity { peer_id, repo_id });
    }

    pub fn writer_peer_id_for(&self, repo_id: &RepoId) -> Option<PeerId> {
        self.writer_identity
            .as_ref()
            .filter(|writer| &writer.repo_id == repo_id)
            .map(|writer| writer.peer_id.clone())
    }

    pub fn clear_sync_binding(&mut self) {
        self.authenticated_peer_id = None;
        self.bound_repo_id = None;
        self.writer_identity = None;
        self.current_sync_scope_nonce = None;
    }

    pub fn is_repo_bound(&self, repo_id: &RepoId) -> bool {
        self.bound_repo_id.as_ref() == Some(repo_id)
    }

    pub fn switch_branch(&mut self, peer_id: Option<String>) {
        self.active_branch = peer_id.map(PeerId::new);
    }

    pub fn set_scope_nonce(&mut self, scope_nonce: Option<u64>) {
        if let Some(scope_nonce) = scope_nonce {
            self.current_scope_nonce = scope_nonce;
        }
    }

    pub fn scope_nonce(&self) -> u64 {
        self.current_scope_nonce
    }

    pub fn set_sync_scope_nonce(&mut self, scope_nonce: u64) {
        self.current_sync_scope_nonce = Some(scope_nonce);
    }

    pub fn sync_scope_nonce(&self) -> Option<u64> {
        self.current_sync_scope_nonce
    }

    pub fn set_active_db(&mut self, handle: DatabaseHandle) {
        self.active_db = Some(handle);
    }

    pub fn clear_active_db(&mut self) {
        self.active_db = None;
    }

    pub fn has_runtime_scope_binding(&self) -> bool {
        self.active_db.is_some()
            || self.authenticated_peer_id.is_some()
            || self.bound_repo_id.is_some()
            || self.current_sync_scope_nonce.is_some()
            || self.writer_identity.is_some()
    }

    /// active_db 为 None 时返回 false（非只读），但这不代表"可写"——
    /// 调用方必须额外检查 `is_repo_bound` 或 `active_db.is_some()` 才能确认写权限。
    pub fn is_readonly(&self) -> bool {
        self.active_db.as_ref().map(|h| h.readonly).unwrap_or(false)
    }

    pub fn get_active_db(&self) -> Option<&DatabaseHandle> {
        self.active_db.as_ref()
    }

    pub fn active_db_for(
        &self,
        branch: Option<&PeerId>,
        repo_name: &str,
        repo_id: Option<RepoId>,
    ) -> Option<&DatabaseHandle> {
        self.active_db.as_ref().filter(|handle| {
            handle.branch.as_ref() == branch && active_db_matches_scope(handle, repo_name, repo_id)
        })
    }
}

pub(super) fn active_db_matches_scope(
    handle: &DatabaseHandle,
    repo_name: &str,
    repo_id: Option<RepoId>,
) -> bool {
    match (repo_id, handle.repo_id) {
        (Some(expected), Some(active)) => active == expected,
        (Some(_), None) => false,
        (None, _) => handle.repo_name.as_str() == repo_name,
    }
}
