//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::build_state;
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_get_changes, session::WsSession,
};
use deve_core::models::DocId;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{staging, ChangeStatus};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_changes_keeps_same_path_entries_for_distinct_doc_ids() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let deleted_doc = DocId::new();
    let added_doc = DocId::new();
    state.repo.run_on_local_repo(state.repo.local_repo_name(), |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/reused.md".into(),
                renamed_from: None,
                doc_id: Some(deleted_doc),
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/reused.md".into(),
                renamed_from: None,
                doc_id: Some(added_doc),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("new"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(23));
    session.switch_repo(state.repo.local_repo_name().to_string(), state.repo.get_repo_info()?.map(|info| info.uuid));
    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ChangesList {
            scope_nonce,
            staged,
            unstaged,
            ..
        }) => {
            assert_eq!(scope_nonce, Some(23));
            assert!(staged.iter().any(|entry| {
                entry.path == "notes/reused.md"
                    && entry.doc_id == Some(deleted_doc)
                    && entry.status == ChangeStatus::Deleted
            }));
            assert!(unstaged.iter().any(|entry| {
                entry.path == "notes/reused.md"
                    && entry.doc_id == Some(added_doc)
                    && entry.status == ChangeStatus::Added
            }));
        }
        other => panic!("expected ChangesList, got {:?}", other),
    }
    Ok(())
}
