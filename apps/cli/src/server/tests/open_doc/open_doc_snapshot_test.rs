use super::handlers::document::handle_open_doc;
use super::{AppState, channel::DualChannel, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, FactActor, Op, PeerId};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    let repo = Arc::new(repo);
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
        default_id,
    ))
}

fn seed_doc(state: &Arc<AppState>, path: &str, content: &str) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo("default", path, None, "test")?;
    state
        .repo
        .local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            "default",
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
        )?;
    let snapshot_seq = state
        .repo
        .get_local_ops_in_local_repo("default", doc_id)?
        .last()
        .map(|(seq, _)| *seq)
        .expect("seeded op seq");
    state
        .repo
        .save_snapshot_in_local_repo("default", doc_id, snapshot_seq, content)?;
    Ok(doc_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_deleted_doc_with_saved_snapshot_returns_error_without_snapshot() -> anyhow::Result<()>
{
    let (_dir, state, default_id) = build_state()?;
    let doc_id = seed_doc(&state, "notes/a.md", "hello")?;
    state.repo.apply_file_delete_structure_in_local_repo(
        "default",
        "notes/a.md",
        Some(doc_id),
        "test",
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.switch_repo("default".into(), Some(default_id));

    handle_open_doc(&state, &ch, &mut session, doc_id, 99).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::DocNotFound);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(uni_rx.try_recv().is_err(), "must not send stale snapshot");
    Ok(())
}
