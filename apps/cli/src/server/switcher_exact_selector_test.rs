use super::handlers::switcher::{handle_switch_branch, handle_switch_repo};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_keeps_exact_remote_selector_without_repo_uuid() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let peer_id = PeerId::new("peer-remote");
    let first = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    let second = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-b".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &first)?;
    repo.ensure_shadow_repo_info(&peer_id, &second)?;
    let selector = repo
        .find_remote_repo_selector_by_id(&peer_id, second.uuid)?
        .expect("collision-safe selector");
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    let state = Arc::new(AppState {
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
    });
    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(selector.clone(), None);

    handle_switch_branch(&state, &ch, &mut session, Some(peer_id.to_string()), None).await;

    assert_eq!(session.active_branch, Some(peer_id));
    assert_eq!(session.active_repo.as_deref(), Some(selector.as_str()));
    assert_eq!(session.active_repo_id, Some(second.uuid));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_fails_closed_when_exact_remote_selector_conflicts_with_repo_id()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let peer_id = PeerId::new("peer-remote");
    let first = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    let second = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-b".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &first)?;
    repo.ensure_shadow_repo_info(&peer_id, &second)?;
    let selector = repo
        .find_remote_repo_selector_by_id(&peer_id, second.uuid)?
        .expect("collision-safe selector");
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    let state = Arc::new(AppState {
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
    });
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));

    handle_switch_repo(
        &state,
        &ch,
        &mut session,
        "wiki".into(),
        Some(second.uuid),
        Some(5),
    )
    .await;

    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce: Some(5),
            ..
        }) if error.code == deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            && error.detail.as_deref().is_some_and(|detail| {
                detail.contains("Session repo mismatch:")
                    && detail.contains(selector.as_str())
            })
    ));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_does_not_emit_partial_repo_view_when_tree_reset_fails() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    let tree_manager = Arc::new(RepoTreeRegistry::new());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = tree_manager.with_tree_mut(uuid::Uuid::new_v4(), None, |_| {
            panic!("poison tree registry")
        });
    }));
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
        tx,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            identity_key.peer_id(),
            repo,
            SyncMode::Auto,
        )),
        tree_manager,
        #[cfg(feature = "search")]
        search_service: None,
        identity_key,
    });
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_switch_repo(&state, &ch, &mut session, "default".into(), None, Some(11)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::RequestFailed
            );
            assert_eq!(switch_nonce, Some(11));
        }
        other => panic!("expected tree rebuild ProtocolError, got {:?}", other),
    }
    assert!(
        uni_rx.try_recv().is_err(),
        "must not emit partial repo view"
    );
    Ok(())
}
