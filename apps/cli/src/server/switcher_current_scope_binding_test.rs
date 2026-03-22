use super::handlers::switcher::handle_switch_branch;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::TempDir;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, PeerId)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let local_info = repo.get_repo_info()?.expect("default repo info");
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(&peer_id, &local_info)?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                identity_key.peer_id(),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key,
        }),
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
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
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
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
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
