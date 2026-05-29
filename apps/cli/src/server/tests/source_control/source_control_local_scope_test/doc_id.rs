//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::{build_state, write_workspace_file};
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_get_doc_diff, session::WsSession,
};
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_diff_rejects_reused_path_when_doc_id_misses() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    write_workspace_file(&dir, "notes/a.md", "hello");
    state
        .repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")?;
    let tracked_doc_id = state.repo.get_docid("notes/a.md")?.expect("tracked doc id");
    state.repo.append_generated_op_in_local_repo(
        "default",
        tracked_doc_id,
        PeerId::new("test"),
        |seq| {
            LedgerEntry::new_content(
                tracked_doc_id,
                Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1,
                PeerId::new("test"),
                seq,
                None,
                None,
            )
        },
    )?;
    state.repo.apply_file_delete_structure_in_local_repo(
        "default",
        "notes/a.md",
        Some(tracked_doc_id),
        "test",
    )?;
    let (reused_doc_id, _ops) = state.repo.apply_file_structure_in_local_repo(
        "default",
        "notes/reused.md",
        None,
        "test",
    )?;
    state.repo.append_generated_op_in_local_repo(
        "default",
        reused_doc_id,
        PeerId::new("test"),
        |seq| {
            LedgerEntry::new_content(
                reused_doc_id,
                Op::Insert {
                    pos: 0,
                    content: "other".into(),
                },
                1,
                PeerId::new("test"),
                seq,
                None,
                None,
            )
        },
    )?;
    write_workspace_file(&dir, "notes/reused.md", "other");

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
        "req-2".into(),
        ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(tracked_doc_id),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScDocNotFound);
            assert_eq!(error.detail.as_deref(), Some("notes/a.md"));
            assert_eq!(scope_nonce, Some(19));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}
