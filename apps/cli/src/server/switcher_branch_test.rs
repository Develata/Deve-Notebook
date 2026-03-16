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
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<Arc<AppState>> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok(Arc::new(AppState {
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
    }))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_unknown_shadow_peer() -> anyhow::Result<()> {
    let state = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some("missing-peer".into()),
        Some(11),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(11));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_local_repo_selector() -> anyhow::Result<()> {
    let state = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_switch_branch(&state, &ch, &mut session, Some("default".into()), Some(13)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(13));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_peer_with_only_broken_shadow_repos() -> anyhow::Result<()> {
    let state = build_state()?;
    let bad_peer = PeerId::new("peer-bad");
    let bad_dir = state.repo.remotes_dir().join(bad_peer.to_filename());
    std::fs::create_dir_all(&bad_dir)?;
    std::fs::write(bad_dir.join("broken.redb"), b"not-a-redb")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(bad_peer.to_string()),
        Some(23),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(switch_nonce, Some(23));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_accepts_shadow_peer_even_if_local_repo_stem_matches() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let local = RepoManager::init(dir.path(), 10, Some("peer-remote"), Some("urn:local:peer"))?;
    let local_info = local.get_repo_info()?.expect("local peer repo info");
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_binding(&peer_id, local_info.uuid)?;
    let repo = Arc::new(repo);
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

    handle_switch_branch(&state, &ch, &mut session, Some(peer_id.to_string()), None).await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::BranchSwitched { success: true, .. })
    ));
    assert_eq!(session.active_branch, Some(peer_id));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_emits_scope_messages_after_success_ack() -> anyhow::Result<()> {
    let state = build_state()?;
    let local = state
        .repo
        .get_repo_info()?
        .expect("local repo info must exist");
    let peer_id = PeerId::new("peer-remote");
    state
        .repo
        .ensure_shadow_repo_binding(&peer_id, local.uuid)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(17),
    )
    .await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::BranchSwitched {
            success: true,
            switch_nonce: Some(17),
            ..
        })
    ));
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::RepoList {
            branch: Some(_),
            ..
        })
    ));
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::RepoSwitched {
            branch: Some(_),
            switch_nonce: Some(17),
            ..
        })
    ));
    Ok(())
}
