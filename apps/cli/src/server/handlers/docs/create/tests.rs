use super::{handle_create_doc, handle_create_doc_request};
use crate::server::tree_state::RepoTreeRegistry;
use crate::server::{AppState, security};
use crate::server::{channel::DualChannel, session::WsSession};
use deve_core::config::SyncMode;
use deve_core::models::{DocId, NodeId, PeerId};
use deve_core::protocol::{
    DocumentCreateRequest, DocumentCreateResponse, ScopeNonce, ServerErrorCode, ServerMessage,
};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let cataloged = crate::test_support::init_cataloged_repo(&ledger, &projection_base, 10)?;
    let repo_id = cataloged.repo_id;
    let repo = Arc::new(cataloged.repo);
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_rejects_empty_name_fail_closed() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_create_doc(&state, &ch, &mut session, "".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::DocumentCreate(DocumentCreateResponse::Rejected { error, .. })) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert!(error.detail.is_none());
        }
        other => panic!("expected typed Document Create rejection, got {:?}", other),
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
            .local_repo_workspace_path(state.repo.local_repo_name(), "notes/trimmed.md")?
            .exists()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_normalizes_backslash_path_before_storage() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_create_doc(&state, &ch, &mut session, "notes\\win".into()).await;

    assert!(state.repo.get_docid("notes/win.md")?.is_some());
    assert!(
        state
            .repo
            .local_repo_workspace_path(state.repo.local_repo_name(), "notes/win.md")?
            .exists()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_rejects_backslash_internal_segment_before_ledger() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_create_doc(&state, &ch, &mut session, "notes\\.notegit\\hidden".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::DocumentCreate(DocumentCreateResponse::Rejected { error, .. })) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert!(error.detail.is_none());
        }
        other => panic!("expected typed Document Create rejection, got {:?}", other),
    }
    assert!(state.repo.get_docid("notes/.notegit/hidden.md")?.is_none());
    assert!(
        !state
            .repo
            .local_repo_workspace_root(state.repo.local_repo_name())?
            .join("notes/.notegit/hidden.md")
            .exists()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn document_create_same_uuid_same_target_replays_without_duplicate_facts_and_conflicts_fail_closed()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));
    let proposed_node_id = NodeId::new();
    let request = DocumentCreateRequest {
        proposed_node_id,
        repo_id,
        branch: None,
        scope_nonce: ScopeNonce::new(0),
        path: "notes/idempotent".into(),
    };

    handle_create_doc_request(&state, &ch, &mut session, request.clone()).await;
    assert_created(
        uni_rx.recv().await.expect("first typed Create response"),
        proposed_node_id,
        "notes/idempotent.md",
    );
    let first_seq = state.repo.run_on_local_repo(
        state.repo.local_repo_name(),
        deve_core::ledger::range::get_max_seq,
    )?;

    handle_create_doc_request(&state, &ch, &mut session, request.clone()).await;
    assert_created(
        uni_rx.recv().await.expect("replayed typed Create response"),
        proposed_node_id,
        "notes/idempotent.md",
    );
    let replay_seq = state.repo.run_on_local_repo(
        state.repo.local_repo_name(),
        deve_core::ledger::range::get_max_seq,
    )?;
    assert_eq!(replay_seq, first_seq, "replay must not append facts");

    let mut mismatched_path = request.clone();
    mismatched_path.path = "notes/other.md".into();
    handle_create_doc_request(&state, &ch, &mut session, mismatched_path).await;
    assert_rejected_conflict(uni_rx.recv().await.expect("UUID/path conflict"));

    let mut mismatched_identity = request;
    mismatched_identity.proposed_node_id = NodeId::new();
    handle_create_doc_request(&state, &ch, &mut session, mismatched_identity).await;
    assert_rejected_conflict(uni_rx.recv().await.expect("path/UUID conflict"));
    assert_eq!(
        state.repo.get_docid("notes/idempotent.md")?,
        Some(DocId(proposed_node_id.0))
    );
    Ok(())
}

fn assert_created(message: ServerMessage, proposed_node_id: NodeId, expected_path: &str) {
    match message {
        ServerMessage::DocumentCreate(DocumentCreateResponse::Created {
            context,
            node_id,
            doc_id,
            path,
            ..
        }) => {
            assert_eq!(context.proposed_node_id, proposed_node_id);
            assert_eq!(node_id, proposed_node_id);
            assert_eq!(doc_id, Some(DocId(proposed_node_id.0)));
            assert_eq!(path, expected_path);
        }
        other => panic!("expected typed Document Create success, got {other:?}"),
    }
}

fn assert_rejected_conflict(message: ServerMessage) {
    match message {
        ServerMessage::DocumentCreate(DocumentCreateResponse::Rejected { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::StorageConflict);
            assert!(error.detail.is_none());
        }
        other => panic!("expected typed Document Create conflict, got {other:?}"),
    }
}
