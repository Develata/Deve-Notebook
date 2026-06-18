//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::{build_state, grant_default_browser_write, write_workspace_file};
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_commit, session::WsSession,
};
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_commit_ack_carries_scope_nonce() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    state
        .repo
        .ensure_local_repo_workspace_identity(state.repo.local_repo_name())?;
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

    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut rx = state.tx.subscribe();
    let mut session = WsSession::new();
    session.switch_repo("default".into(), None);
    grant_default_browser_write(&state, &mut session, 23)?;

    handle_commit(&state, &ch, &mut session, "initial".into()).await;

    match rx.recv().await.expect("broadcast ack") {
        ServerMessage::CommitAck {
            repo_id,
            scope_nonce,
            ..
        } => {
            assert_eq!(repo_id, state.repo.get_repo_info()?.map(|info| info.uuid));
            assert_eq!(scope_nonce, Some(23));
        }
        other => panic!("expected CommitAck, got {:?}", other),
    }
    Ok(())
}
