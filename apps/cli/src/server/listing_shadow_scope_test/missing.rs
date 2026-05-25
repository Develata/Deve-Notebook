//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::support::{bind_stale_runtime, build_state};
use crate::server::{
    channel::DualChannel, handlers::listing::handle_list_shadows, session::WsSession,
};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_shadows_on_missing_remote_branch_clears_stale_scope() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(21));
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), Some(default_id));
    bind_stale_runtime(&state, &mut session, default_id, 21)?;

    handle_list_shadows(&state, &ch, Some(&mut session), Some("req-shadow".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScStaleScope);
        }
        other => panic!(
            "expected ProtocolError for stale shadow scope, got {:?}",
            other
        ),
    }
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_shadows_on_missing_remote_preserves_scope_nonce_in_error() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(77));
    session.switch_branch(Some("ghost-peer".into()));

    handle_list_shadows(&state, &ch, Some(&mut session), Some("req-scope".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(77));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.active_branch.is_none());
    Ok(())
}
