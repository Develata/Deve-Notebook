use super::handlers::document::{handle_open_doc, handle_request_history};
use super::handlers::listing::handle_list_docs;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
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

fn seed_shadow_doc(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
) -> anyhow::Result<DocId> {
    state.repo.ensure_shadow_repo_info(
        peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:shadow-notes".into()),
        },
    )?;
    let doc_id = DocId::new();
    state
        .repo
        .run_on_shadow_repo_by_id(peer_id, &repo_id, |db| {
            let _ = deve_core::ledger::node_meta::ensure_file_node(db, "notes/a.md", doc_id)?;
            let entry = LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "hello remote".into(),
                },
                1,
                peer_id.clone(),
                1,
                None,
                None,
            );
            let bytes = bincode::serialize(&entry)?;
            let write = db.begin_write()?;
            write
                .open_table(LEDGER_OPS)?
                .insert(1u64, bytes.as_slice())?;
            write
                .open_multimap_table(DOC_OPS)?
                .insert(doc_id.as_u128(), 1u64)?;
            write.commit()?;
            Ok(())
        })?;
    Ok(doc_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_doc_on_remote_branch_uses_shadow_repo_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let doc_id = seed_shadow_doc(&state, &peer_id, test_repo_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_repo_id));

    handle_open_doc(&state, &ch, &mut session, doc_id, 9).await;

    match uni_rx.recv().await {
        Some(ServerMessage::Snapshot {
            repo_id,
            doc_id: seen_doc,
            content,
            ..
        }) => {
            assert_eq!(repo_id, test_repo_id);
            assert_eq!(seen_doc, doc_id);
            assert_eq!(content, "hello remote");
        }
        other => panic!("expected Snapshot, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_history_on_remote_branch_uses_shadow_repo_without_locked_db() -> anyhow::Result<()>
{
    let (_dir, state, test_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let doc_id = seed_shadow_doc(&state, &peer_id, test_repo_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_repo_id));

    handle_request_history(&state, &ch, &mut session, doc_id, 11).await;

    match uni_rx.recv().await {
        Some(ServerMessage::History { repo_id, ops, .. }) => {
            assert_eq!(repo_id, test_repo_id);
            assert_eq!(ops.len(), 1);
        }
        other => panic!("expected History, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_on_remote_branch_uses_shadow_repo_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let _ = seed_shadow_doc(&state, &peer_id, test_repo_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_repo_id));

    handle_list_docs(&state, &ch, &mut session, None, None).await;

    loop {
        match uni_rx.recv().await {
            Some(ServerMessage::DocList { repo_id, docs, .. }) => {
                assert_eq!(repo_id, Some(test_repo_id));
                assert_eq!(docs.len(), 1);
                assert_eq!(docs[0].1, "notes/a.md");
                break;
            }
            Some(_) => continue,
            None => panic!("expected DocList"),
        }
    }
    Ok(())
}
