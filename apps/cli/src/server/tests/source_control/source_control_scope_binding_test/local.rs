//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::build_state;
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_get_changes, session::WsSession,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_changes_without_repo_selection_bootstrap_after_clearing_stale_runtime_binding(
) -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(13);

    handle_get_changes(&state, &ch, &mut session, Some("req-local-miss".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ChangesList { repo_id, .. }) => {
            assert_eq!(repo_id, Some(default_id));
        }
        other => panic!("expected ChangesList, got {:?}", other),
    }
    assert_eq!(session.active_repo.as_deref(), Some(state.repo.local_repo_name()));
    assert_eq!(session.active_repo_id, Some(default_id));
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_changes_without_repo_selection_bootstrap_single_repo() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_get_changes(
        &state,
        &ch,
        &mut session,
        Some("req-local-bootstrap".into()),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ChangesList { repo_id, .. }) => {
            assert_eq!(repo_id, Some(default_id));
        }
        other => panic!("expected ChangesList, got {:?}", other),
    }
    assert_eq!(session.active_repo.as_deref(), Some(state.repo.local_repo_name()));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}
