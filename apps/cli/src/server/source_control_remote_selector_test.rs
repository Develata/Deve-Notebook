use super::handlers::source_control::{handle_get_changes, handle_get_commit_history};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, PeerId, uuid::Uuid, String)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
        tx: broadcast::channel(16).0,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            PeerId::new("test-peer"),
            repo.clone(),
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_service: None,
        identity_key: security::load_or_generate_identity_key(&dir.path().join("host"))?,
    });
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:test:wiki-a".into()),
        },
    )?;
    let second = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-b".into()),
    };
    state.repo.ensure_shadow_repo_info(&peer_id, &second)?;
    let selector = state
        .repo
        .find_remote_repo_selector_by_id(&peer_id, second.uuid)?
        .expect("collision-safe selector");
    Ok((dir, state, peer_id, second.uuid, selector))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_changes_recover_collision_safe_selector_from_uuid() -> anyhow::Result<()> {
    let (_dir, state, peer_id, repo_id, selector) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("wiki".into(), Some(repo_id));

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ChangesList {
            repo_id: seen_repo, ..
        }) => assert_eq!(seen_repo, Some(repo_id)),
        other => panic!("expected ChangesList, got {:?}", other),
    }
    assert_eq!(session.active_repo.as_deref(), Some(selector.as_str()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_history_recovers_collision_safe_selector_from_uuid() -> anyhow::Result<()>
{
    let (_dir, state, peer_id, repo_id, selector) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("wiki".into(), Some(repo_id));

    handle_get_commit_history(&state, &ch, &mut session, "req-1".into(), 10).await;

    match uni_rx.recv().await {
        Some(ServerMessage::CommitHistory {
            repo_id: seen_repo, ..
        }) => assert_eq!(seen_repo, Some(repo_id)),
        other => panic!("expected CommitHistory, got {:?}", other),
    }
    assert_eq!(session.active_repo.as_deref(), Some(selector.as_str()));
    Ok(())
}
