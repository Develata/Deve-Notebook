use super::handlers::document::handle_edit;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::CLIENT_OP_INDEX;
use deve_core::models::{DocId, Op, PeerId};
use deve_core::protocol::ServerMessage;
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
        test_id,
    ))
}

fn seed_doc(state: &Arc<AppState>, repo_name: &str, path: &str) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
    Ok(doc_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_repairs_missing_client_op_index_for_secondary_local_repo() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "test", "notes/a.md")?;
    state.repo.run_on_local_repo("test", |db| {
        let write = db.begin_write()?;
        let _ = write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(31));
    session.switch_repo("test".into(), Some(test_repo_id));
    session.set_writer_identity(test_repo_id, PeerId::new("writer"));

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
            doc_id: ack_doc,
            client_op_id,
            ..
        }) => {
            assert_eq!(scope_nonce, Some(31));
            assert_eq!(ack_doc, doc_id);
            assert_eq!(client_op_id, 9);
        }
        other => panic!("expected Ack after runtime repair, got {:?}", other),
    }
    Ok(())
}
