use super::handlers::switcher::handle_switch_repo;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

fn browser_session(scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_exact_fails_closed_when_remote_repo_id_is_stale() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let peer_id = PeerId::new("peer-remote");
    let info = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &info)?;
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
    let mut session = browser_session(30);
    session.switch_branch(Some(peer_id.to_string()));

    handle_switch_repo(
        &state,
        &ch,
        &mut session,
        "wiki".into(),
        Some(uuid::Uuid::new_v4()),
        Some(31),
    )
    .await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce: Some(31),
            ..
        }) if error.code == ServerErrorCode::ScRepoContextInvalid
            && error.detail.as_deref().is_some_and(|detail| {
                detail.contains("Repository UUID not resolved for exact repository selector wiki")
            })
    ));
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    Ok(())
}
