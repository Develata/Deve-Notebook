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

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
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
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_on_unbound_shadow_branch_clears_stale_db_and_sync_binding() -> anyhow::Result<()>
{
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), Some(default_id));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(11);

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
async fn list_repos_on_unbound_shadow_branch_clears_stale_db_and_sync_binding() -> anyhow::Result<()>
{
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), Some(default_id));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(11);

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
    let shadow_peer = PeerId::new("peer-a");
    let shadow_repo = uuid::Uuid::new_v4();
    state
        .repo
        .ensure_shadow_repo_binding(&shadow_peer, shadow_repo)?;
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
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
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
