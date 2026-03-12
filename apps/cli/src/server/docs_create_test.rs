use super::handlers::docs::handle_create_doc;
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
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_rejects_existing_workspace_file_without_backfill() -> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    let path = dir.path().join("vault/default/external.md");
    std::fs::create_dir_all(path.parent().expect("parent"))?;
    std::fs::write(&path, "external only")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_create_doc(&state, &ch, &mut session, "external.md".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error }) => {
            assert_eq!(error.code, ServerErrorCode::StorageConflict);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(state.repo.get_docid("external.md")?.is_none());
    assert_eq!(std::fs::read_to_string(path)?, "external only");
    Ok(())
}
