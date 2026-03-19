use super::handlers::listing::{handle_list_docs, handle_list_repos};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo_id = repo.get_repo_info()?.expect("default info").uuid;
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
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key,
        }),
        repo_id,
    ))
}

fn seed_stale_binding(session: &mut WsSession, state: &Arc<AppState>, repo_id: uuid::Uuid) {
    session.set_active_db(
        state
            .repo
            .open_database(None, state.repo.local_repo_name())
            .expect("local handle"),
    );
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(7);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_rejects_unbound_local_scope_with_stale_runtime_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    seed_stale_binding(&mut session, &state, repo_id);

    handle_list_docs(
        &state,
        &ch,
        &mut session,
        Some("req-local-docs".into()),
        None,
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.active_repo.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_repos_rejects_unbound_local_scope_with_stale_runtime_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    seed_stale_binding(&mut session, &state, repo_id);

    handle_list_repos(&state, &ch, &mut session, Some("req-local-repos".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.active_repo.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
