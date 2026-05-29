//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::support::{bind_stale_runtime, build_state, seed_shadow_repo};
use crate::server::{
    channel::DualChannel, handlers::listing::handle_list_shadows, session::WsSession,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_shadows_on_clean_remote_branch_still_succeeds() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_shadow_repo(&state, &peer_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(34));
    session.switch_branch(Some(peer_id.to_string()));

    handle_list_shadows(&state, &ch, Some(&mut session), Some("req-shadow".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ShadowList {
            request_id,
            scope_nonce,
            shadows,
        }) => {
            assert_eq!(request_id.as_deref(), Some("req-shadow"));
            assert_eq!(scope_nonce, Some(34));
            assert_eq!(shadows, vec![peer_id.to_string()]);
        }
        other => panic!(
            "expected ShadowList for clean remote scope, got {:?}",
            other
        ),
    }
    assert_eq!(session.active_branch.as_ref(), Some(&peer_id));
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_shadows_on_clean_local_scope_clears_stale_runtime_binding() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let peer_id = PeerId::new("peer-a");
    seed_shadow_repo(&state, &peer_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(88));
    bind_stale_runtime(&state, &mut session, default_id, 88)?;

    handle_list_shadows(&state, &ch, Some(&mut session), Some("req-shadow".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ShadowList {
            request_id,
            scope_nonce,
            shadows,
        }) => {
            assert_eq!(request_id.as_deref(), Some("req-shadow"));
            assert_eq!(scope_nonce, Some(88));
            assert_eq!(shadows, vec![peer_id.to_string()]);
        }
        other => panic!("expected ShadowList for clean local scope, got {:?}", other),
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
