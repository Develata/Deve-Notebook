use super::handlers::document::handle_open_doc;
use super::open_doc_invalid_delta_test_support::inject_legacy_invalid_insert;
use super::{AppState, channel::DualChannel, security, tree_state::RepoTreeRegistry};
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
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let host_dir = dir.path().join("host");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    let repo = Arc::new(repo);
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx: broadcast::channel(16).0,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key: security::load_or_generate_identity_key(&host_dir)?,
        }),
        default_id,
    ))
}

fn seed_doc(state: &Arc<AppState>, path: &str, content: &str) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
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
async fn open_doc_rebuilds_full_snapshot_when_delta_ops_are_out_of_bounds() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    let doc_id = seed_doc(&state, "notes/a.md", "hi")?;
    inject_legacy_invalid_insert(&state, doc_id, PeerId::new("test-peer"))?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.switch_repo("default".into(), Some(default_id));

    handle_open_doc(&state, &ch, &mut session, doc_id, 7).await;

    match uni_rx.recv().await {
        Some(ServerMessage::Snapshot {
            content,
            base_seq,
            version,
            delta_ops,
            ..
        }) => {
            assert_eq!(content, "hi!");
            assert_eq!(base_seq, version);
            assert!(base_seq > 0);
            assert!(delta_ops.is_empty(), "must not send invalid delta chain");
        }
        other => panic!("expected Snapshot, got {:?}", other),
    }
    Ok(())
}
