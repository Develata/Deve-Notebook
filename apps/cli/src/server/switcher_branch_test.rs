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

    handle_switch_branch(&state, &ch, &mut session, Some("missing-peer".into()), None).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
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

    handle_switch_branch(&state, &ch, &mut session, Some("default".into()), None).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_accepts_shadow_peer_even_if_local_display_name_matches() -> anyhow::Result<()>
{
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let local = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:notes"))?;
    local.run_on_local_repo("notes", |db| {
        let read = db.begin_read()?;
        let table = read.open_table(REPO_METADATA)?;
        let raw = table.get(&0)?.expect("repo info");
        let mut info: deve_core::ledger::RepoInfo = bincode::deserialize(raw.value())?;
        info.name = "peer-remote".into();
        drop(table);
        drop(read);
        let write = db.begin_write()?;
        {
            let mut table = write.open_table(REPO_METADATA)?;
            table.insert(&0, bincode::serialize(&info)?.as_slice())?;
        }
        write.commit()?;
        Ok(())
    })?;
    let local = local
        .get_repo_info_for(None, Some("notes"))?
        .expect("local repo info");
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_binding(&peer_id, local.uuid)?;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_keeps_repo_unbound_when_target_branch_is_ambiguous() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let default_info = repo.get_repo_info()?.expect("default info");
    let mut notes_repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:notes"))?;
    notes_repo.set_vault_root(&vault);
    let notes_info = notes_repo.get_repo_info()?.expect("notes info");
    let mut ghost_repo = RepoManager::init(dir.path(), 10, Some("ghost"), Some("urn:ghost"))?;
    ghost_repo.set_vault_root(&vault);
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
            repo.clone(),
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_service: None,
        identity_key,
    });
    let peer_id = PeerId::new("peer-remote");
    state
        .repo
        .ensure_shadow_repo_binding(&peer_id, default_info.uuid)?;
    state
        .repo
        .ensure_shadow_repo_binding(&peer_id, notes_info.uuid)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("ghost".into(), None);

    handle_switch_branch(&state, &ch, &mut session, Some(peer_id.to_string()), None).await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::BranchSwitched { success: true, .. })
    ));
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::RepoList {
            branch: Some(_),
            repos,
            ..
        }) if repos.len() == 2
    ));
    assert!(
        uni_rx.try_recv().is_err(),
        "ambiguous target must not auto-switch repo"
    );
    assert_eq!(session.active_branch, Some(peer_id));
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_keeps_repo_unbound_when_only_same_name_repo_has_different_url()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let local = RepoManager::init(dir.path(), 10, Some("wiki"), Some("urn:local:wiki"))?;
    let local_info = local.get_repo_info()?.expect("local wiki info");
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:remote:wiki".into()),
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
    session.switch_repo("wiki".into(), Some(local_info.uuid));

    handle_switch_branch(&state, &ch, &mut session, Some(peer_id.to_string()), None).await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::BranchSwitched { success: true, .. })
    ));
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::RepoList { repos, .. }) if repos == vec!["wiki".to_string()]
    ));
    assert!(
        uni_rx.try_recv().is_err(),
        "same-name but different-url remote repo must not auto-bind"
    );
    assert_eq!(session.active_branch, Some(peer_id));
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}
