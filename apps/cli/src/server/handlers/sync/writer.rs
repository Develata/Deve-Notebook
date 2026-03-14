use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};

pub(super) fn handle(
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    peer_id: PeerId,
    scope_nonce: u64,
) {
    match validate(session, repo_id, &peer_id, scope_nonce) {
        Ok(()) => {
            session.set_writer_identity(repo_id, peer_id.clone());
            ch.unicast(ServerMessage::WriteReady {
                peer_id,
                repo_id,
                scope_nonce,
                branch: session.active_branch.clone(),
            });
        }
        Err(error) => ch.send_protocol_error(error),
    }
}

fn validate(
    session: &WsSession,
    repo_id: RepoId,
    peer_id: &PeerId,
    scope_nonce: u64,
) -> Result<(), ServerError> {
    if session.is_readonly() {
        return Err(ServerError::new(ServerErrorCode::ScRemoteBranchReadonly));
    }
    if session.is_browser_session() {
        if session.active_branch.is_some() || session.active_repo_id != Some(repo_id) {
            return Err(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                "writer scope does not match active repo",
            ));
        }
        if session.scope_nonce() != scope_nonce || session.sync_scope_nonce() != Some(scope_nonce) {
            return Err(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                "writer scope nonce is stale",
            ));
        }
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
    use deve_core::ledger::database::DatabaseHandle;
    use std::sync::Arc;

    #[test]
    fn rejects_unbound_repo() {
        let session = WsSession::new();
        let error = validate(&session, uuid::Uuid::nil(), &PeerId::new("browser"), 1).unwrap_err();
        assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
    }

    #[test]
    fn rejects_readonly_writer_registration() {
        let mut session = WsSession::new();
        let repo_id = uuid::Uuid::new_v4();
        let peer_id = PeerId::new("browser");
        let dir = tempfile::tempdir().expect("tempdir");
        let db = Arc::new(redb::Database::create(dir.path().join("remote.redb")).expect("db"));
        session.set_active_db(DatabaseHandle {
            db,
            readonly: true,
            branch: Some(PeerId::new("remote")),
            repo_id: None,
            repo_name: "repo".into(),
        });
        session.set_authenticated(peer_id.clone());
        let error = validate(&session, repo_id, &peer_id, 1).unwrap_err();
        assert_eq!(error.code, ServerErrorCode::ScRemoteBranchReadonly);
    }

    #[test]
    fn rejects_mismatched_peer() {
        let mut session = WsSession::new();
        let repo_id = uuid::Uuid::new_v4();
        session.set_authenticated(PeerId::new("browser-a"));
        session.bind_repo(repo_id);
        let error = validate(&session, repo_id, &PeerId::new("browser-b"), 1).unwrap_err();
        assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    }

    #[test]
    fn accepts_bound_matching_peer() {
        let mut session = WsSession::new();
        let repo_id = uuid::Uuid::new_v4();
        let peer_id = PeerId::new("browser-a");
        session.set_authenticated(peer_id.clone());
        session.bind_repo(repo_id);
        assert!(validate(&session, repo_id, &peer_id, 1).is_ok());
    }

    #[test]
    fn rejects_browser_writer_with_stale_scope_nonce() {
        let mut session = WsSession::new();
        let repo_id = uuid::Uuid::new_v4();
        let peer_id = PeerId::new("browser-a");
        session.mark_browser_session();
        session.switch_repo("notes".into(), Some(repo_id));
        session.set_scope_nonce(Some(9));
        session.set_sync_scope_nonce(9);
        session.set_authenticated(peer_id.clone());
        session.bind_repo(repo_id);
        let error = validate(&session, repo_id, &peer_id, 8).unwrap_err();
        assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn rejects_browser_writer_for_non_active_repo() {
        let mut session = WsSession::new();
        let repo_id = uuid::Uuid::new_v4();
        let peer_id = PeerId::new("browser-a");
        session.mark_browser_session();
        session.switch_repo("notes".into(), Some(uuid::Uuid::new_v4()));
        session.set_scope_nonce(Some(9));
        session.set_sync_scope_nonce(9);
        session.set_authenticated(peer_id.clone());
        session.bind_repo(repo_id);
        let error = validate(&session, repo_id, &peer_id, 9).unwrap_err();
        assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    }
}
