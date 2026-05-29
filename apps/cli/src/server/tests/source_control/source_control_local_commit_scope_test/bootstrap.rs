//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::{build_state, write_workspace_file};
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_commit, session::WsSession,
};
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_commit_bootstraps_after_clearing_stale_runtime_binding() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    write_workspace_file(&dir, "notes/stale.md", "hello");
    state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/stale.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello"),
                    detected_at: 1,
                    has_conflict: false,
                },
            )
        })?;
    state.repo.stage_pending("notes/stale.md")?;

    let stale_db = Arc::new(redb::Database::create(dir.path().join("stale-local.redb"))?);
    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut rx = state.tx.subscribe();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(29));
    session.set_active_db(DatabaseHandle {
        db: stale_db,
        readonly: false,
        branch: None,
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "ghost".into(),
    });
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(29);

    handle_commit(&state, &ch, &mut session, "stale".into()).await;

    match rx.recv().await.expect("broadcast ack") {
        ServerMessage::CommitAck {
            repo_id,
            scope_nonce,
            ..
        } => {
            assert_eq!(repo_id, state.repo.get_repo_info()?.map(|info| info.uuid));
            assert_eq!(scope_nonce, Some(29));
        }
        other => panic!("expected CommitAck, got {:?}", other),
    }
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
