use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::RepoId;

pub(super) fn require_bound_repo_id(ch: &DualChannel, session: &WsSession) -> Option<RepoId> {
    match session.bound_repo_id {
        Some(repo_id) => Some(repo_id),
        None => {
            ch.send_error("No repository bound to session".to_string());
            None
        }
    }
}
