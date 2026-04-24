//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::support::{bind_stale_shadow_scope, build_state};
use crate::server::{channel::DualChannel, handlers::listing::handle_list_docs, session::WsSession};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_on_unbound_shadow_branch_clears_stale_db_and_sync_binding() -> anyhow::Result<()>
{
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    bind_stale_shadow_scope(&state, &mut session, default_id, 11)?;

    handle_list_docs(&state, &ch, &mut session, None, None).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
        }
        other => panic!("expected SyncRepoUnbound error, got {:?}", other),
    }
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_on_unbound_shadow_branch_preserves_switch_nonce() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("missing-shadow".into()));

    handle_list_docs(&state, &ch, &mut session, Some("req-1".into()), Some(17)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(17));
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Remote branch not available:"))
            );
        }
        other => panic!("expected ProtocolError with switch nonce, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_does_not_emit_partial_repo_view_when_tree_reset_fails() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = state
            .tree_manager
            .with_tree_mut(uuid::Uuid::new_v4(), None, |_| {
                panic!("poison tree registry")
            });
    }));
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_list_docs(&state, &ch, &mut session, Some("req-tree".into()), Some(41)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(switch_nonce, Some(41));
        }
        other => panic!("expected tree rebuild ProtocolError, got {:?}", other),
    }
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(
        uni_rx.try_recv().is_err(),
        "must not emit partial repo view"
    );
    Ok(())
}
