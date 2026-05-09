//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Session-derived broadcast scope matching.

use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ScopeNonce;

#[derive(Clone, Default)]
pub(super) struct SessionBroadcastScope {
    pub(super) browser_session: bool,
    pub(super) active_repo_id: Option<RepoId>,
    pub(super) active_branch: Option<PeerId>,
    pub(super) scope_nonce: ScopeNonce,
}

impl SessionBroadcastScope {
    pub(super) fn from_session(session: &WsSession) -> Self {
        Self {
            browser_session: session.is_browser_session(),
            active_repo_id: session.active_repo_id,
            active_branch: session.active_branch.clone(),
            scope_nonce: ScopeNonce::new(session.scope_nonce()),
        }
    }
}

pub(super) fn matches_scope(
    active_repo_id: Option<RepoId>,
    active_branch: Option<&PeerId>,
    message_repo_id: &Option<RepoId>,
    message_branch: Option<&PeerId>,
    local_only: bool,
) -> bool {
    if local_only && active_branch.is_some() {
        return false;
    }
    match (active_repo_id, message_repo_id, message_branch) {
        (None, Some(_), _) => false,
        (Some(_), None, _) => false,
        (Some(active_repo_id), Some(message_repo_id), Some(branch)) => {
            active_repo_id == *message_repo_id && active_branch == Some(branch)
        }
        (Some(active_repo_id), Some(message_repo_id), None) => {
            active_repo_id == *message_repo_id && active_branch.is_none()
        }
        _ => true,
    }
}

pub(super) fn matches_runtime_scope_nonce(
    current_scope_nonce: ScopeNonce,
    message_scope_nonce: Option<u64>,
) -> bool {
    current_scope_nonce.matches_optional(message_scope_nonce)
}
