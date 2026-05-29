//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::build_state;
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_get_changes, session::WsSession,
};
use deve_core::models::DocId;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, staging};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_changes_keeps_unstaged_entry_after_staging_same_doc_path() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let doc_id = DocId::new();
    state.repo.run_on_local_repo("default", |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("staged"),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("edited-again"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(29));
    session.switch_repo("default".into(), None);
    handle_get_changes(&state, &ch, &mut session, Some("req-2".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ChangesList {
            staged, unstaged, ..
        }) => {
            assert!(staged.iter().any(|entry| {
                entry.path == "notes/a.md"
                    && entry.doc_id == Some(doc_id)
                    && entry.status == ChangeStatus::Added
            }));
            assert!(unstaged.iter().any(|entry| {
                entry.path == "notes/a.md"
                    && entry.doc_id == Some(doc_id)
                    && entry.status == ChangeStatus::Added
            }));
        }
        other => panic!("expected ChangesList, got {:?}", other),
    }
    Ok(())
}
