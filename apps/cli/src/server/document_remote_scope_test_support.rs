//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime

use super::{AppState, session::WsSession};
use deve_core::ledger::{RepoInfo, schema::{DOC_OPS, LEDGER_OPS}};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use tokio::sync::mpsc;

const DOC_PATH: &str = "notes/a.md";
const SHADOW_REPO_NAME: &str = "shadow-notes";

pub(super) fn seed_shadow_doc(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: RepoId,
) -> anyhow::Result<DocId> {
    state.repo.ensure_shadow_repo_info(
        peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: SHADOW_REPO_NAME.into(),
            url: Some("urn:shadow-notes".into()),
        },
    )?;
    let doc_id = DocId::new();
    state.repo.run_on_shadow_repo_by_id(peer_id, &repo_id, |db| {
        let _ = deve_core::ledger::node_meta::ensure_file_node(db, DOC_PATH, doc_id)?;
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
        write.open_table(LEDGER_OPS)?.insert(1u64, bytes.as_slice())?;
        write.open_multimap_table(DOC_OPS)?.insert(doc_id.as_u128(), 1u64)?;
        write.commit()?;
        Ok(())
    })?;
    Ok(doc_id)
}

pub(super) fn remote_browser_session(
    peer_id: &PeerId,
    repo_id: RepoId,
    scope_nonce: u64,
) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(SHADOW_REPO_NAME.into(), Some(repo_id));
    session
}

pub(super) async fn assert_snapshot(
    rx: &mut mpsc::Receiver<ServerMessage>,
    repo_id: RepoId,
    doc_id: DocId,
    scope_nonce: u64,
) {
    match rx.recv().await {
        Some(ServerMessage::Snapshot {
            repo_id: seen_repo,
            scope_nonce: seen_scope,
            doc_id: seen_doc,
            content,
            ..
        }) => {
            assert_eq!(seen_repo, repo_id);
            assert_eq!(seen_scope, Some(scope_nonce));
            assert_eq!(seen_doc, doc_id);
            assert_eq!(content, "hello remote");
        }
        other => panic!("expected Snapshot, got {:?}", other),
    }
}

pub(super) async fn assert_history(
    rx: &mut mpsc::Receiver<ServerMessage>,
    repo_id: RepoId,
    scope_nonce: u64,
) {
    match rx.recv().await {
        Some(ServerMessage::History {
            repo_id: seen_repo,
            scope_nonce: seen_scope,
            ops,
            ..
        }) => {
            assert_eq!(seen_repo, repo_id);
            assert_eq!(seen_scope, Some(scope_nonce));
            assert_eq!(ops.len(), 1);
        }
        other => panic!("expected History, got {:?}", other),
    }
}

pub(super) async fn assert_doc_list(rx: &mut mpsc::Receiver<ServerMessage>, repo_id: RepoId) {
    loop {
        match rx.recv().await {
            Some(ServerMessage::DocList {
                repo_id: seen_repo,
                docs,
                ..
            }) => {
                assert_eq!(seen_repo, Some(repo_id));
                assert_eq!(docs.len(), 1);
                assert_eq!(docs[0].1, DOC_PATH);
                break;
            }
            Some(_) => continue,
            None => panic!("expected DocList"),
        }
    }
}
