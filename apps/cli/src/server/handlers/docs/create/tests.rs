use super::handle_create_doc;
use crate::server::tree_state::RepoTreeRegistry;
use crate::server::{AppState, security};
use crate::server::{channel::DualChannel, session::WsSession};
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
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let (tx, _rx) = broadcast::channel(16);
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
        repo_id,
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_rejects_empty_name_fail_closed() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_create_doc(&state, &ch, &mut session, "".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(error.detail.as_deref(), Some("Invalid empty path"));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(state.repo.get_docid(".md")?.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_trims_outer_whitespace_before_appending_md() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_create_doc(&state, &ch, &mut session, "  notes/trimmed  ".into()).await;

    assert!(state.repo.get_docid("notes/trimmed.md")?.is_some());
    assert!(
        state
            .repo
            .local_repo_workspace_path("default", "notes/trimmed.md")?
            .exists()
    );
    Ok(())
}
