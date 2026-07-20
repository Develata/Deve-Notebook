//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime

use super::session::WsSession;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

pub(super) fn stale_local_binding_session(path: &Path) -> anyhow::Result<WsSession> {
    let mut session = WsSession::new();
    session.set_active_db(DatabaseHandle {
        db: Arc::new(redb::Database::create(path.join("stale-local.redb"))?),
        readonly: false,
        branch: None,
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "ghost".into(),
    });
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(41);
    Ok(session)
}

pub(super) async fn assert_snapshot(
    rx: &mut mpsc::Receiver<ServerMessage>,
    repo_id: RepoId,
    doc_id: DocId,
) {
    match rx.recv().await {
        Some(ServerMessage::Snapshot {
            repo_id: seen_repo,
            doc_id: seen_doc,
            ..
        }) => {
            assert_eq!(seen_repo, repo_id);
            assert_eq!(seen_doc, doc_id);
        }
        other => panic!("expected Snapshot, got {:?}", other),
    }
}

pub(super) async fn assert_history(
    rx: &mut mpsc::Receiver<ServerMessage>,
    repo_id: RepoId,
    doc_id: DocId,
) {
    match rx.recv().await {
        Some(ServerMessage::History {
            repo_id: seen_repo,
            doc_id: seen_doc,
            ..
        }) => {
            assert_eq!(seen_repo, repo_id);
            assert_eq!(seen_doc, doc_id);
        }
        other => panic!("expected History, got {:?}", other),
    }
}

pub(super) fn assert_bootstrapped_session(session: &WsSession, repo_id: RepoId) {
    assert_eq!(
        session.active_repo.as_deref(),
        Some(repo_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(repo_id));
}

pub(super) fn assert_stale_binding_cleared(session: &WsSession) {
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
}
