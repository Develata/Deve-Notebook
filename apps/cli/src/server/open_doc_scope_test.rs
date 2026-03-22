use super::handlers::document::{handle_open_doc, handle_request_history};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let mut test_repo = RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    test_repo.set_vault_root(&vault);
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let repo = Arc::new(repo);
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
        test_id,
    ))
}

fn seed_doc(
    state: &Arc<AppState>,
    repo_name: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
    state.repo.append_generated_op_in_local_repo(
        repo_name,
        doc_id,
        PeerId::new("test-peer"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("test-peer"),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(doc_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_doc_on_wrong_repo_returns_error_without_empty_snapshot() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "notes/a.md", "hello")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(7));
    session.switch_repo("test".into(), Some(test_repo_id));

    handle_open_doc(&state, &ch, &mut session, doc_id, 7).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { scope_nonce, .. }) => {
            assert_eq!(scope_nonce, Some(7));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(uni_rx.try_recv().is_err(), "must not send empty snapshot");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_deleted_doc_returns_error_without_snapshot() -> anyhow::Result<()> {
    let (_dir, state, _test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "notes/a.md", "hello")?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    state.repo.apply_file_delete_structure_in_local_repo(
        state.repo.local_repo_name(),
        "notes/a.md",
        Some(doc_id),
        "test",
    )?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("default".into(), Some(default_id));

    handle_open_doc(&state, &ch, &mut session, doc_id, 8).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::StorageNotFound);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(uni_rx.try_recv().is_err(), "must not send deleted snapshot");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_history_on_wrong_repo_returns_error_without_history() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "notes/a.md", "hello")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(9));
    session.switch_repo("test".into(), Some(test_repo_id));

    handle_request_history(&state, &ch, &mut session, doc_id, 9).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { scope_nonce, .. }) => {
            assert_eq!(scope_nonce, Some(9));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(uni_rx.try_recv().is_err(), "must not send empty history");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_history_on_deleted_doc_returns_error_without_history() -> anyhow::Result<()> {
    let (_dir, state, _test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "notes/a.md", "hello")?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    state.repo.apply_file_delete_structure_in_local_repo(
        state.repo.local_repo_name(),
        "notes/a.md",
        Some(doc_id),
        "test",
    )?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("default".into(), Some(default_id));

    handle_request_history(&state, &ch, &mut session, doc_id, 10).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::StorageNotFound);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(uni_rx.try_recv().is_err(), "must not send deleted history");
    Ok(())
}
