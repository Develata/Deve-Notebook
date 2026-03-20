use super::handlers::switcher::handle_switch_branch;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_local_scope_hint_is_raw_uuid_string()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let local = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:notes"))?;
    let local_info = local.get_repo_info()?.expect("local notes info");
    let peer_id = PeerId::new("peer-remote");
    let remote_repo_id = uuid::Uuid::new_v4();
    repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: remote_repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:notes".into()),
        },
    )?;
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
    session.switch_repo(local_info.uuid.to_string(), None);

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(29),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(29));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert!(session.active_repo.is_none());
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_local_scope_metadata_is_broken()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let local_info = repo.get_repo_info()?.expect("default repo info");
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(&peer_id, &local_info)?;
    let db = repo.open_database(None, "default")?.db;
    let txn = db.begin_write()?;
    txn.open_table(REPO_METADATA)?
        .insert(&0, [0_u8, 1, 2, 3].as_slice())?;
    txn.commit()?;

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
    session.switch_repo("default".into(), Some(local_info.uuid));

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(41),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(switch_nonce, Some(41));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    Ok(())
}
