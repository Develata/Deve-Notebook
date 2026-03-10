use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};

pub(super) fn handle(ch: &DualChannel, session: &mut WsSession, repo_id: RepoId, peer_id: PeerId) {
    match validate(session, repo_id, &peer_id) {
        Ok(()) => {
            session.set_writer_identity(repo_id, peer_id.clone());
            ch.unicast(ServerMessage::WriteReady { peer_id, repo_id });
        }
        Err(error) => ch.send_protocol_error(error),
    }
}

fn validate(session: &WsSession, repo_id: RepoId, peer_id: &PeerId) -> Result<(), ServerError> {
    if session.is_readonly() {
        return Err(ServerError::new(ServerErrorCode::SyncEditRejected));
    }
    if !session.is_repo_bound(&repo_id) {
        return Err(ServerError::new(ServerErrorCode::SyncRepoUnbound));
    }
    let Some(auth_peer_id) = session.authenticated_peer_id.as_ref() else {
        return Err(ServerError::new(ServerErrorCode::SyncPeerUnauthenticated));
    };
    if auth_peer_id != peer_id {
        return Err(ServerError::with_detail(
            ServerErrorCode::SyncPeerUnauthenticated,
            "writer peer mismatch",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unbound_repo() {
        let session = WsSession::new();
        let error = validate(&session, uuid::Uuid::nil(), &PeerId::new("browser")).unwrap_err();
        assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
    }

    #[test]
    fn rejects_mismatched_peer() {
        let mut session = WsSession::new();
        let repo_id = uuid::Uuid::new_v4();
        session.set_authenticated(PeerId::new("browser-a"));
        session.bind_repo(repo_id);
        let error = validate(&session, repo_id, &PeerId::new("browser-b")).unwrap_err();
        assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    }

    #[test]
    fn accepts_bound_matching_peer() {
        let mut session = WsSession::new();
        let repo_id = uuid::Uuid::new_v4();
        let peer_id = PeerId::new("browser-a");
        session.set_authenticated(peer_id.clone());
        session.bind_repo(repo_id);
        assert!(validate(&session, repo_id, &peer_id).is_ok());
    }
}
