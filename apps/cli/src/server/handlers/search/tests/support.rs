use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::ledger::RepoInfo;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId, serialize_ledger_entry};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use tokio::sync::mpsc;

pub(crate) fn session_for_repo(repo_name: &str, repo_id: uuid::Uuid) -> WsSession {
    let mut session = WsSession::new();
    session.switch_repo(repo_name.into(), Some(repo_id));
    session
}

pub(crate) fn test_channel(state: &Arc<AppState>) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
    let (tx, rx) = mpsc::channel(8);
    (DualChannel::new(state.tx.clone(), tx), rx)
}

pub(crate) fn seed_remote_doc_with_content(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    path: &str,
    content: &str,
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
            let _ = deve_core::ledger::node_meta::ensure_file_node(db, path, doc_id)?;
            let entry = LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                peer_id.clone(),
                1,
                None,
                None,
            );
            let bytes = serialize_ledger_entry(&entry)?;
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

pub(crate) async fn assert_scoped_empty_results(
    rx: &mut mpsc::Receiver<ServerMessage>,
    expected_request_id: &str,
    expected_repo_id: uuid::Uuid,
    expected_scope_nonce: Option<u64>,
) {
    match rx.recv().await {
        Some(ServerMessage::SearchResults {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            results,
        }) => {
            assert_eq!(request_id, expected_request_id);
            assert_eq!(repo_id, Some(expected_repo_id));
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, expected_scope_nonce);
            assert!(results.is_empty());
        }
        other => panic!("expected empty SearchResults, got {:?}", other),
    }
}

pub(crate) fn search_enabled_state(source: &Arc<AppState>) -> Arc<AppState> {
    Arc::new(AppState {
        repo: source.repo.clone(),
        sync_manager: source.sync_manager.clone(),
        tx: source.tx.clone(),
        plugins: Vec::new(),
        sync_engine: source.sync_engine.clone(),
        tree_manager: source.tree_manager.clone(),
        search_available: true,
        identity_key: source.identity_key.clone(),
        git_bridge: source.git_bridge,
    })
}
