use super::handlers::document::handle_edit;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoManager, database::DatabaseHandle};
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
    let test_id = repo.get_repo_info()?.expect("repo info").uuid;
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

fn seed_doc(state: &Arc<AppState>) -> anyhow::Result<DocId> {
    let (doc_id, _ops) =
        state
            .repo
            .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")?;
    state.repo.append_generated_op_in_local_repo(
        "default",
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
    Ok(doc_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_clears_stale_remote_readonly_binding_before_checks() -> anyhow::Result<()> {
    let (dir, state, default_repo_id) = build_state()?;
    let doc_id = seed_doc(&state)?;
    let stale_db = Arc::new(redb::Database::create(dir.path().join("stale-remote.redb"))?);
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("default".into(), Some(default_repo_id));
    session.set_scope_nonce(Some(29));
    session.set_authenticated(PeerId::new("writer"));
    session.bind_repo(default_repo_id);
    session.set_writer_identity(default_repo_id, PeerId::new("writer"));
    session.set_active_db(DatabaseHandle {
        db: stale_db,
        readonly: true,
        branch: Some(PeerId::new("remote")),
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "shadow".into(),
    });

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
        Some(ServerMessage::EditRejected {
            scope_nonce,
            doc_id: rejected_doc_id,
            client_op_id,
            error,
        }) => {
            assert_eq!(scope_nonce, Some(29));
            assert_eq!(rejected_doc_id, doc_id);
            assert_eq!(client_op_id, 9);
            assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
        }
        other => panic!(
            "expected EditRejected(SyncPeerUnauthenticated), got {:?}",
            other
        ),
    }
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.writer_identity.is_none());
    Ok(())
}
