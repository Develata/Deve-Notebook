//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::support::{build_state, seed_remote_branch, seed_stale_binding};
use crate::server::{
    channel::DualChannel, handlers::listing::handle_list_repos, session::WsSession,
};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_repos_rejects_unbound_remote_scope_with_stale_runtime_binding() -> anyhow::Result<()>
{
    let (_dir, state, repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_branch(&state, &peer_id, repo_id);
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    seed_stale_binding(&mut session, &state, repo_id);

    handle_list_repos(&state, &ch, &mut session, Some("req-remote-repos".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.starts_with("stale remote scope:"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch.as_ref(), Some(&peer_id));
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
