//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::{
    AppState, session::WsSession,
    sync_scope_cleanup_test_support::assert_runtime_binding_cleared,
};
use deve_core::ledger::RepoInfo;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use tokio::sync::mpsc;

const SHADOW_REPO_NAME: &str = "wiki";

pub(super) fn ensure_shadow_repo(
    state: &AppState,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
) -> anyhow::Result<()> {
    state.repo.ensure_shadow_repo_info(
        peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: SHADOW_REPO_NAME.into(),
            url: Some("urn:test:wiki".into()),
        },
    )
}

pub(super) fn browser_session(scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

pub(super) fn active_peer_session(
    state: &AppState,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
    auth_peer: PeerId,
) -> anyhow::Result<WsSession> {
    let mut session = browser_session(scope_nonce);
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(SHADOW_REPO_NAME.into(), Some(repo_id));
    session.set_active_db(state.repo.open_database(Some(peer_id), SHADOW_REPO_NAME)?);
    session.set_authenticated(auth_peer);
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(scope_nonce);
    Ok(session)
}

pub(super) async fn assert_shadow_list(
    rx: &mut mpsc::Receiver<ServerMessage>,
    expected_scope_nonce: u64,
    expect_empty: bool,
) {
    let Some(ServerMessage::ShadowList {
        request_id,
        scope_nonce,
        shadows,
    }) = rx.recv().await
    else {
        panic!("expected scoped ShadowList");
    };
    assert_eq!(scope_nonce, Some(expected_scope_nonce));
    if expect_empty {
        assert_eq!(request_id, None);
        assert!(shadows.is_empty());
    }
}

pub(super) fn assert_remote_scope_cleared(session: &WsSession) {
    assert_eq!(session.active_branch, None);
    assert!(session.active_repo.is_none());
    assert_runtime_binding_cleared(session);
}
