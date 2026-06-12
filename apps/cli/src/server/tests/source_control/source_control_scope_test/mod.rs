use super::handlers::source_control::{
    handle_get_changes, handle_get_commit_diff, handle_get_commit_history, handle_get_doc_diff,
};
use super::{channel::DualChannel, session::WsSession};
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::{ScPathTarget, ServerMessage};
use deve_core::source_control::SourceControlApi;
use tokio::sync::mpsc;

use super::source_control_scope_test_support as support;
use support::{build_state, recv_commit_diff, recv_history, seed_pending, write_workspace_file};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_changes_rejects_stale_local_selector() -> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    seed_pending(state.repo.as_ref(), "test", "notes/a.md", "hello");
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn commit_history_rejects_stale_local_selector() -> anyhow::Result<()> {
    let (dir, state, default_id, test_id) = build_state()?;
    write_workspace_file(&dir, "test", "notes/a.md", "hello");
    seed_pending(state.repo.as_ref(), "test", "notes/a.md", "hello");
    let selector = RepoSelector {
        repo_id: Some(test_id),
        repo_name: Some("test".into()),
    };
    state
        .repo
        .stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    state.repo.commit_staged_in_repo_with_git_bridge(
        &selector,
        "initial",
        deve_core::config::GitBridgeMode::Mirror,
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));

    handle_get_commit_history(&state, &ch, &mut session, "req-1".into(), 10).await;
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

mod extra;
mod write_extra;
