use super::handlers::document::handle_edit;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, Op, PeerId};
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc, mpsc::error::TryRecvError};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
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
    ))
}

fn seed_doc(state: &Arc<AppState>, path: &str) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo("default", path, None, "test")?;
    Ok(doc_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_acknowledges_ledger_commit_when_workspace_writeback_fails() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let doc_id = seed_doc(&state, "notes/a.md")?;
    std::fs::create_dir_all(dir.path().join("vault/default/notes/a.md"))?;
    let default_repo_id = state.repo.get_repo_info()?.expect("repo info").uuid;
    let mut broadcast_rx = state.tx.subscribe();
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(37));
    session.switch_repo("default".into(), Some(default_repo_id));
    session.set_writer_identity(default_repo_id, PeerId::new("writer"));

    handle_edit(
        &state,
        &ch,
        &mut session,
        doc_id,
        Op::Insert {
            pos: 0,
            content: "!".into(),
        },
        7,
        9,
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::Ack {
            scope_nonce,
            doc_id: ack_doc_id,
            client_op_id,
            ..
        }) => {
            assert_eq!(scope_nonce, Some(37));
            assert_eq!(ack_doc_id, doc_id);
            assert_eq!(client_op_id, 9);
        }
        other => panic!("expected Ack after ledger commit, got {:?}", other),
    }
    match broadcast_rx.recv().await? {
        ServerMessage::NewOp {
            doc_id: broadcast_doc_id,
            ..
        } => assert_eq!(broadcast_doc_id, doc_id),
        other => panic!("expected NewOp broadcast, got {:?}", other),
    }
    assert!(matches!(uni_rx.try_recv(), Err(TryRecvError::Empty)));
    assert!(
        state
            .repo
            .find_client_op_in_local_repo("default", doc_id, 7, 9)?
            .is_some()
    );
    Ok(())
}
