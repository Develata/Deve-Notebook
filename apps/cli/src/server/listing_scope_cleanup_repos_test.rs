//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::support::{bind_stale_shadow_scope, build_state, seed_shadow_repo};
use crate::server::{channel::DualChannel, handlers::listing::handle_list_repos, session::WsSession};
use deve_core::ledger::RepoManager;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_repos_on_unbound_shadow_branch_clears_stale_db_and_sync_binding() -> anyhow::Result<()>
{
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    bind_stale_shadow_scope(&state, &mut session, default_id, 11)?;

    handle_list_repos(&state, &ch, &mut session, Some("req-2".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
        }
        other => panic!("expected ProtocolError after cleanup, got {:?}", other),
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
async fn list_repos_on_clean_unbound_shadow_branch_succeeds() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (shadow_peer, shadow_repo) = seed_shadow_repo(&state)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(shadow_peer.to_string()));

    handle_list_repos(&state, &ch, &mut session, Some("req-remote-repos".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::RepoList {
            request_id,
            branch,
            repos,
            ..
        }) => {
            assert_eq!(request_id.as_deref(), Some("req-remote-repos"));
            assert_eq!(branch.as_deref(), Some(shadow_peer.as_str()));
            assert_eq!(repos, vec![shadow_repo.to_string()]);
        }
        other => panic!(
            "expected RepoList for clean unbound shadow branch, got {:?}",
            other
        ),
    }
    assert_eq!(session.active_branch.as_ref(), Some(&shadow_peer));
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_repos_rejects_stale_local_selector_and_clears_session() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));

    handle_list_repos(&state, &ch, &mut session, Some("req-stale".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
        }
        other => panic!(
            "expected stale local selector ProtocolError, got {:?}",
            other
        ),
    }
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    Ok(())
}
