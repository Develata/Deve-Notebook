use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::RepoId;

use super::errors;

pub(super) fn require_bound_repo_id(ch: &DualChannel, session: &WsSession) -> Option<RepoId> {
    match session.bound_repo_id {
        Some(repo_id) => Some(repo_id),
        None => {
            errors::repo_unbound(ch);
            None
        }
    }
}
