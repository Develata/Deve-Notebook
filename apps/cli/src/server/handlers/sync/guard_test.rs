//! plan_ref:
//!   - 05_network#server-ws-runtime

use super::validate_browser_sync_scope;
use crate::server::session::WsSession;
use deve_core::protocol::ServerErrorCode;

#[test]
fn rejects_missing_browser_sync_scope() {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(9));

    let error = validate_browser_sync_scope(&mut session).expect_err("must fail");
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_sync_cleared(&session);
}

#[test]
fn rejects_stale_browser_sync_scope() {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(11));
    session.set_sync_scope_nonce(7);

    let error = validate_browser_sync_scope(&mut session).expect_err("must fail");
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_sync_cleared(&session);
}

fn assert_sync_cleared(session: &WsSession) {
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
}
