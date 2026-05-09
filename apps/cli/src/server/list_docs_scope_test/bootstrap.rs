//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::support::{build_single_repo_state, seed_doc};
use crate::server::{
    channel::DualChannel, handlers::listing::handle_list_docs, session::WsSession,
};
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_with_stale_local_binding_bootstraps_single_repo() -> anyhow::Result<()> {
    let (dir, state, default_id) = build_single_repo_state()?;
    let stale_db = Arc::new(redb::Database::create(dir.path().join("stale-local.redb"))?);
    let doc_id = seed_doc(&state, "default", "notes/a.md", "hello")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.set_active_db(DatabaseHandle {
        db: stale_db,
        readonly: false,
        branch: None,
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "ghost".into(),
    });
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(17);

    handle_list_docs(
        &state,
        &ch,
        &mut session,
        Some("req-bootstrap".into()),
        None,
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::RepoSwitched { uuid, .. }) => {
            assert_eq!(uuid, default_id.to_string());
        }
        other => panic!("expected RepoSwitched, got {:?}", other),
    }
    match uni_rx.recv().await {
        Some(ServerMessage::DocList { repo_id, docs, .. }) => {
            assert_eq!(repo_id, Some(default_id));
            assert!(docs.iter().any(|(seen, _)| *seen == doc_id));
        }
        other => panic!("expected DocList, got {:?}", other),
    }
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
