use super::authorize_browser_local_write_with_grants;
use crate::server::session::WsSession;
use crate::server::source_control_grants::{
    AuthSessionId, SourceControlGrantBranch, SourceControlWriteGrants,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;
use std::time::Duration;

fn live_browser_writer(
    repo_id: uuid::Uuid,
    auth_session_id: AuthSessionId,
    peer_id: PeerId,
    scope_nonce: u64,
) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.bind_auth_session(auth_session_id);
    session.switch_repo("default".into(), Some(repo_id));
    session.set_scope_nonce(Some(scope_nonce));
    session.set_sync_scope_nonce(scope_nonce);
    session.set_authenticated(peer_id.clone());
    session.bind_repo(repo_id);
    session.mark_sync_hello_accepted();
    session.set_writer_identity(repo_id, peer_id, scope_nonce);
    session
}

#[test]
fn ws_source_control_live_writer_renews_expired_http_grant() {
    let grants = SourceControlWriteGrants::with_ttl(Duration::from_millis(25));
    let auth_session_id = AuthSessionId::for_test("live-writer-renewal");
    let repo_id = uuid::Uuid::new_v4();
    let peer_id = PeerId::new("browser-writer");
    let session = live_browser_writer(repo_id, auth_session_id.clone(), peer_id.clone(), 7);
    grants
        .grant(
            auth_session_id.clone(),
            repo_id,
            SourceControlGrantBranch::Local,
            peer_id.clone(),
            7,
        )
        .unwrap();
    std::thread::sleep(Duration::from_millis(40));
    assert!(
        grants
            .authorize_browser_local(&auth_session_id, repo_id, 7)
            .is_err()
    );

    authorize_browser_local_write_with_grants(&grants, &session, repo_id).unwrap();

    assert_eq!(
        grants
            .authorize_browser_local(&auth_session_id, repo_id, 7)
            .unwrap(),
        peer_id
    );
}

#[test]
fn ws_source_control_grant_refresh_rejects_stale_writer_binding() {
    let grants = SourceControlWriteGrants::new();
    let auth_session_id = AuthSessionId::for_test("stale-writer-renewal");
    let repo_id = uuid::Uuid::new_v4();
    let mut session = live_browser_writer(
        repo_id,
        auth_session_id.clone(),
        PeerId::new("browser-writer"),
        7,
    );
    session.set_writer_identity(repo_id, PeerId::new("other-writer"), 7);

    let error = authorize_browser_local_write_with_grants(&grants, &session, repo_id)
        .expect_err("mismatched live writer must not refresh the grant");

    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
    assert!(
        grants
            .authorize_browser_local(&auth_session_id, repo_id, 7)
            .is_err()
    );
}
