//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::support::{build_state, seed_stale_binding};
use crate::server::{channel::DualChannel, handlers::listing, session::WsSession};
use deve_core::protocol::ServerMessage;
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_bootstrap_single_repo_after_clearing_stale_runtime_binding() -> anyhow::Result<()>
{
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    seed_stale_binding(&mut session, &state, repo_id);

    listing::handle_list_docs(
        &state,
        &ch,
        &mut session,
        Some("req-local-docs".into()),
        None,
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::RepoSwitched {
            repo_id: actual_repo_id,
            ..
        }) => {
            assert_eq!(actual_repo_id, repo_id);
        }
        other => panic!("expected RepoSwitched, got {:?}", other),
    }
    assert_eq!(
        session.active_repo.as_deref(),
        Some(state.repo.local_repo_name())
    );
    assert_eq!(session.active_repo_id, Some(repo_id));
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_repos_clear_unbound_local_scope_with_stale_runtime_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    seed_stale_binding(&mut session, &state, repo_id);

    listing::handle_list_repos(&state, &ch, &mut session, Some("req-local-repos".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::RepoList { repo_entries, .. }) => {
            assert_eq!(repo_entries.len(), 1);
            assert_eq!(repo_entries[0].display_alias, "default");
        }
        other => panic!("expected RepoList, got {:?}", other),
    }
    assert!(session.active_repo.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
