use super::handlers::listing::handle_list_shadows;
use super::{
    channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry, AppState,
};
use deve_core::config::SyncMode;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _repo_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        }),
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_shadows_rejects_stale_local_selector_scope() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("ghost".into(), Some(default_id));

    handle_list_shadows(&state, &ch, Some(&mut session), Some("req-shadow".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_shadows_on_broken_remote_branch_preserves_branch_scope() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let peer_id = PeerId::new("peer-broken");
    std::fs::write(
        state.repo.remotes_dir().join(peer_id.to_filename()),
        b"not-a-directory",
    )?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(33));
    session.switch_branch(Some(peer_id.to_string()));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(33);

    handle_list_shadows(&state, &ch, Some(&mut session), Some("req-shadow".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(scope_nonce, Some(33));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch.as_ref(), Some(&peer_id));
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
