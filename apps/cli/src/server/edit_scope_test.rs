use super::handlers::document::handle_edit;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::CLIENT_OP_INDEX;
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

fn seed_doc(
    state: &Arc<AppState>,
    repo_name: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<DocId> {
    let doc_id = state
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
async fn edit_rejects_doc_outside_active_repo_before_append() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "notes/a.md", "hello")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(19));
    session.switch_repo("test".into(), Some(test_repo_id));
    session.set_writer_identity(test_repo_id, PeerId::new("writer"));

    handle_edit(
        &state,
        &ch,
        &mut session,
        doc_id,
        Op::Insert {
            pos: 5,
            content: "!".into(),
        },
        7,
        9,
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::EditRejected { scope_nonce, error }) => {
            assert_eq!(scope_nonce, Some(19));
            assert_eq!(error.code, ServerErrorCode::StorageNotFound);
        }
        other => panic!("expected EditRejected(StorageNotFound), got {:?}", other),
    }
    assert!(
        state
            .repo
            .find_client_op_in_local_repo("test", doc_id, 7, 9)?
            .is_none(),
        "must not append orphan op into active repo"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_fails_closed_on_broken_client_op_index() -> anyhow::Result<()> {
    let (_dir, state, _test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "notes/a.md", "hello")?;
    let default_repo_id = state.repo.get_repo_info()?.expect("repo info").uuid;
    let op_count_before = state.repo.get_local_ops(doc_id)?.len();
    state.repo.run_on_local_repo("default", |db| {
        let write = db.begin_write()?;
        write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(23));
    session.switch_repo("default".into(), Some(default_repo_id));
    session.set_writer_identity(default_repo_id, PeerId::new("writer"));

    handle_edit(
        &state,
        &ch,
        &mut session,
        doc_id,
        Op::Insert {
            pos: 5,
            content: "!".into(),
        },
        7,
        9,
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(scope_nonce, Some(23));
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Broken client op index")),
                "unexpected detail: {:?}",
                error.detail
            );
        }
        other => panic!(
            "expected ProtocolError(StoragePersistFailed), got {:?}",
            other
        ),
    }
    assert_eq!(state.repo.get_local_ops(doc_id)?.len(), op_count_before);
    Ok(())
}
