//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#tree-projection-contract

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::stale_unbound_remote_scope_detail;
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{DocId, NodeId, NodeMeta, RepoId, RepoType};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(super) struct LocalBootstrapGuard {
    should_rollback_repo: bool,
}

impl LocalBootstrapGuard {
    pub(super) fn new(session: &WsSession) -> Self {
        Self {
            should_rollback_repo: session.active_branch.is_none()
                && session.active_repo.is_none()
                && session.active_repo_id.is_none(),
        }
    }

    pub(super) fn rollback_after_error(&self, session: &mut WsSession) {
        if self.should_rollback_repo {
            session.clear_active_repo();
        }
    }
}

pub(super) fn precheck_remote_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
    switch_nonce: Option<u64>,
) -> bool {
    let Some(branch) = session.active_branch.as_ref().cloned() else {
        return false;
    };
    if session.active_repo.is_some() || session.active_repo_id.is_some() {
        return false;
    }
    if let Err(error) = shadow_scope::ensure_remote_branch_available(state, &branch) {
        let error = if error.is_remote_branch_unavailable() {
            state.revoke_source_control_write_grant_for_session(session);
            shadow_scope::clear_stale_remote_branch(session);
            ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, error.detail())
        } else {
            clear_runtime_binding_and_revoke(state, session);
            ServerError::from(error)
        };
        ch.send_protocol_error_with_scope_and_switch_nonce(error, scope_nonce, switch_nonce);
        return true;
    }
    if !session.has_runtime_scope_binding() {
        return false;
    }
    clear_runtime_binding_and_revoke(state, session);
    ch.send_protocol_error_with_scope_and_switch_nonce(
        ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            stale_unbound_remote_scope_detail(&branch),
        ),
        scope_nonce,
        switch_nonce,
    );
    true
}

fn clear_runtime_binding_and_revoke(state: &Arc<AppState>, session: &mut WsSession) {
    state.revoke_source_control_write_grant_for_session(session);
    session.clear_active_db();
    session.clear_sync_binding();
}

pub(super) fn load_docs(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_id: RepoId,
) -> anyhow::Result<Vec<(DocId, String)>> {
    state.repo.list_docs(&resolved_repo_type(session, repo_id))
}

pub(super) fn load_nodes(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_id: RepoId,
) -> anyhow::Result<Vec<(NodeId, NodeMeta)>> {
    state.repo.list_nodes(&resolved_repo_type(session, repo_id))
}

fn resolved_repo_type(session: &WsSession, repo_id: RepoId) -> RepoType {
    match session.active_branch.clone() {
        Some(peer_id) => RepoType::Remote(peer_id, repo_id),
        None => RepoType::Local(repo_id),
    }
}
