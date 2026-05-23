//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::handlers::switcher::handle_switch_branch;
use super::switcher_test_support::{app_state, browser_session, unicast_channel};
use super::{AppState, session::WsSession};
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, PeerId)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos(&vault);
    let local_info = repo.get_repo_info()?.expect("default repo info");
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(&peer_id, &local_info)?;
    let state = app_state(repo, vault, dir.path().join("host"))?;
    Ok((
        dir,
        state,
        local_info.uuid,
        peer_id,
    ))
}

fn seed_stale_runtime_binding(session: &mut WsSession, state: &Arc<AppState>, repo_id: uuid::Uuid) {
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())
        .expect("local handle");
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(19);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_unbound_local_scope_with_stale_runtime_binding() -> anyhow::Result<()>
{
    let (_dir, state, repo_id, peer_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(80);
    seed_stale_runtime_binding(&mut session, &state, repo_id);

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(81),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
            assert_eq!(switch_nonce, Some(81));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_unbound_remote_scope_with_stale_runtime_binding()
-> anyhow::Result<()> {
    let (_dir, state, repo_id, peer_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(81);
    session.switch_branch(Some(peer_id.to_string()));
    seed_stale_runtime_binding(&mut session, &state, repo_id);

    handle_switch_branch(&state, &ch, &mut session, None, Some(82)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.starts_with("stale remote scope:"))
            );
            assert_eq!(switch_nonce, Some(82));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, Some(peer_id));
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
