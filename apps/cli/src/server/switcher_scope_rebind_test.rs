use super::handlers::switcher::handle_switch_branch;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_to_local_rebinds_single_repo_after_stale_remote_scope() -> anyhow::Result<()>
{
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
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
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), None);

    handle_switch_branch(&state, &ch, &mut session, None, Some(17)).await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::BranchSwitched {
            peer_id: None,
            success: true,
            switch_nonce: Some(17),
        })
    ));
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::RepoList { branch: None, repos, .. })
            if repos == vec!["default".to_string()]
    ));
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::RepoSwitched {
            branch: None,
            name,
            uuid,
            switch_nonce: Some(17),
        }) if name == "default" && uuid == default_id.to_string()
    ));
    assert_eq!(session.active_branch, None);
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}
