use super::handle_delete_doc;
use crate::server::tree_state::RepoTreeRegistry;
use crate::server::{AppState, security};
use crate::server::{channel::DualChannel, session::WsSession};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
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
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?);
    sync_manager.scan()?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager,
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

fn seed_file(state: &Arc<AppState>, path: &str) -> anyhow::Result<()> {
    let (doc_id, _ops) = state.repo.apply_file_structure_in_local_repo(
        state.repo.local_repo_name(),
        path,
        None,
        "test",
    )?;
    state.repo.append_generated_op_in_local_repo(
        state.repo.local_repo_name(),
        doc_id,
        PeerId::new("test-peer"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1,
                PeerId::new("test-peer"),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_rejects_empty_path_fail_closed() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_delete_doc(&state, &ch, &mut session, "   ".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(error.detail.as_deref(), Some("Invalid empty path"));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_trims_path_before_resolving_target() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    seed_file(&state, "notes/a.md")?;
    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_delete_doc(&state, &ch, &mut session, "  notes/a.md  ".into()).await;

    assert!(state.repo.get_docid("notes/a.md")?.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_rejects_traversal_path_before_resolving_target() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_delete_doc(&state, &ch, &mut session, "../secret.md".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(error.detail.as_deref(), Some("Invalid path: ../secret.md"));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}
