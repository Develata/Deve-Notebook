//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::{build_state, write_workspace_file};
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_get_doc_diff, session::WsSession,
};
use deve_core::protocol::{ScPathTarget, ServerMessage};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_diff_resolves_renamed_target_before_reading_workspace() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    write_workspace_file(&dir, "notes/a.md", "hello");
    state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello"),
                    detected_at: 1,
                    has_conflict: false,
                },
            )
        })?;
    state.repo.stage_pending("notes/a.md")?;
    state.repo.commit_staged("initial")?;
    let doc_id = state
        .repo
        .get_docid("notes/a.md")?
        .expect("existing doc id");

    std::fs::remove_file(dir.path().join("notes").join("default").join("notes/a.md"))?;
    write_workspace_file(&dir, "notes/b.md", "hello renamed");
    state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: Some(doc_id),
                    change_type: ChangeStatus::Deleted,
                    content_hash: String::new(),
                    detected_at: 2,
                    has_conflict: false,
                },
            )?;
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/b.md".into(),
                    renamed_from: Some("notes/a.md".into()),
                    doc_id: Some(doc_id),
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello renamed"),
                    detected_at: 2,
                    has_conflict: false,
                },
            )
        })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(19));
    session.switch_repo("default".into(), None);

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "req-1".into(),
        ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::DocDiff {
            doc_id: actual_doc_id,
            path,
            old_content,
            new_content,
            ..
        }) => {
            assert_eq!(actual_doc_id, Some(doc_id));
            assert_eq!(path, "notes/b.md");
            assert_eq!(old_content, "hello");
            assert_eq!(new_content, "hello renamed");
        }
        other => panic!("expected DocDiff, got {:?}", other),
    }
    Ok(())
}
