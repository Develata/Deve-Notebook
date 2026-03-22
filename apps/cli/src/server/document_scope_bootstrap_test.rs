use super::handlers::document::{handle_open_doc, handle_request_history};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let host_dir = dir.path().join("host");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    let repo = Arc::new(repo);
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx: broadcast::channel(16).0,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key: security::load_or_generate_identity_key(&host_dir)?,
        }),
        default_id,
    ))
}

fn seed_doc(state: &Arc<AppState>, path: &str, content: &str) -> anyhow::Result<DocId> {
    let doc_id = state
        .repo
        .apply_file_structure_in_local_repo("default", path, None, "test")?;
    state.repo.append_generated_op_in_local_repo(
        "default",
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
async fn open_doc_without_repo_selection_bootstraps_single_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    let doc_id = seed_doc(&state, "notes/a.md", "hello")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_open_doc(&state, &ch, &mut session, doc_id, 1).await;

    match uni_rx.recv().await {
        Some(ServerMessage::Snapshot {
            repo_id,
            doc_id: seen,
            ..
        }) => {
            assert_eq!(repo_id, default_id);
            assert_eq!(seen, doc_id);
        }
        other => panic!("expected Snapshot, got {:?}", other),
    }
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_history_without_repo_selection_bootstraps_single_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    let doc_id = seed_doc(&state, "notes/a.md", "hello")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();

    handle_request_history(&state, &ch, &mut session, doc_id, 2).await;

    match uni_rx.recv().await {
        Some(ServerMessage::History {
            repo_id,
            doc_id: seen,
            ..
        }) => {
            assert_eq!(repo_id, default_id);
            assert_eq!(seen, doc_id);
        }
        other => panic!("expected History, got {:?}", other),
    }
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}
