//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Source-control remote readonly write gate tests.

use super::*;
use crate::server::handlers::source_control::{
    handle_commit, handle_commit_and_push, handle_discard_file, handle_resolve_conflict,
    handle_stage_file, handle_stage_files, handle_unstage_file, handle_unstage_files,
};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::source_control::ConflictResolution;

fn ensure_shadow_repo(repo: &RepoManager, repo_id: RepoId) -> anyhow::Result<PeerId> {
    let peer_id = PeerId::new("peer-a");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:test".into()),
        },
    )?;
    Ok(peer_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_source_control_writes_are_rejected_before_mutation() -> anyhow::Result<()>
{
    let (dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    write_workspace_file(&dir, "test", "notes/pending.md", "pending");
    write_workspace_file(&dir, "test", "notes/staged.md", "staged");
    seed_pending(state.repo.as_ref(), "test", "notes/pending.md", "pending");
    seed_pending(state.repo.as_ref(), "test", "notes/staged.md", "staged");
    state
        .repo
        .stage_pending_in_local_repo("test", "notes/staged.md")?;
    let expected_local_state = local_source_control_counts(state.repo.as_ref(), "test")?;

    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(41));
    let pending_target = || ScPathTarget::from_path("notes/pending.md");
    let staged_target = || ScPathTarget::from_path("notes/staged.md");

    handle_stage_file(&state, &ch, &mut session, pending_target()).await;
    expect_remote_readonly_error(&mut uni_rx, 41).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_stage_files(&state, &ch, &mut session, vec![pending_target()]).await;
    expect_remote_readonly_error(&mut uni_rx, 41).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_unstage_file(&state, &ch, &mut session, staged_target()).await;
    expect_remote_readonly_error(&mut uni_rx, 41).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_unstage_files(&state, &ch, &mut session, vec![staged_target()]).await;
    expect_remote_readonly_error(&mut uni_rx, 41).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_discard_file(&state, &ch, &mut session, pending_target()).await;
    expect_remote_readonly_error(&mut uni_rx, 41).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_resolve_conflict(
        &state,
        &ch,
        &mut session,
        pending_target(),
        ConflictResolution::KeepLedger,
    )
    .await;
    expect_remote_readonly_error(&mut uni_rx, 41).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_commit(&state, &ch, &mut session, "remote write".into()).await;
    expect_remote_readonly_error(&mut uni_rx, 41).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_commit_and_push(&state, &ch, &mut session, "remote write".into()).await;
    expect_remote_readonly_error(&mut uni_rx, 41).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn degraded_local_source_control_writes_are_rejected_before_mutation() -> anyhow::Result<()>
{
    let (dir, state, _default_id, test_id) = build_state()?;
    write_workspace_file(&dir, "test", "notes/pending.md", "pending");
    write_workspace_file(&dir, "test", "notes/staged.md", "staged");
    seed_pending(state.repo.as_ref(), "test", "notes/pending.md", "pending");
    seed_pending(state.repo.as_ref(), "test", "notes/staged.md", "staged");
    state
        .repo
        .stage_pending_in_local_repo("test", "notes/staged.md")?;
    state.sync_manager.mark_projection_writeback_fault("test");
    let expected_local_state = local_source_control_counts(state.repo.as_ref(), "test")?;

    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("test".into(), Some(test_id));
    session.set_scope_nonce(Some(43));
    let pending_target = || ScPathTarget::from_path("notes/pending.md");
    let staged_target = || ScPathTarget::from_path("notes/staged.md");

    handle_stage_file(&state, &ch, &mut session, pending_target()).await;
    expect_degraded_projection_error(&mut uni_rx, 43).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_stage_files(&state, &ch, &mut session, vec![pending_target()]).await;
    expect_degraded_projection_error(&mut uni_rx, 43).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_unstage_file(&state, &ch, &mut session, staged_target()).await;
    expect_degraded_projection_error(&mut uni_rx, 43).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_unstage_files(&state, &ch, &mut session, vec![staged_target()]).await;
    expect_degraded_projection_error(&mut uni_rx, 43).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_discard_file(&state, &ch, &mut session, pending_target()).await;
    expect_degraded_projection_error(&mut uni_rx, 43).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_resolve_conflict(
        &state,
        &ch,
        &mut session,
        pending_target(),
        ConflictResolution::KeepLedger,
    )
    .await;
    expect_degraded_projection_error(&mut uni_rx, 43).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_commit(&state, &ch, &mut session, "degraded write".into()).await;
    expect_degraded_projection_error(&mut uni_rx, 43).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);

    handle_commit_and_push(&state, &ch, &mut session, "degraded write".into()).await;
    expect_degraded_projection_error(&mut uni_rx, 43).await;
    assert_local_source_control_counts(state.repo.as_ref(), "test", expected_local_state);
    Ok(())
}

async fn expect_remote_readonly_error(rx: &mut mpsc::Receiver<ServerMessage>, nonce: u64) {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRemoteBranchReadonly);
            assert_eq!(error.detail.as_deref(), Some("shadow-notes"));
            assert_eq!(scope_nonce, Some(nonce));
        }
        other => panic!("expected remote readonly ProtocolError, got {:?}", other),
    }
}

async fn expect_degraded_projection_error(rx: &mut mpsc::Receiver<ServerMessage>, nonce: u64) {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("projection is degraded"))
            );
            assert_eq!(scope_nonce, Some(nonce));
        }
        other => panic!("expected degraded projection ProtocolError, got {:?}", other),
    }
}

fn local_source_control_counts(
    repo: &RepoManager,
    repo_name: &str,
) -> anyhow::Result<(usize, usize, usize)> {
    Ok((
        repo.list_pending_fs_in_local_repo(repo_name)?.len(),
        repo.list_staged_in_local_repo(repo_name)?.len(),
        repo.list_commits_in_local_repo(repo_name, 10)?.len(),
    ))
}

fn assert_local_source_control_counts(
    repo: &RepoManager,
    repo_name: &str,
    expected: (usize, usize, usize),
) {
    assert_eq!(
        local_source_control_counts(repo, repo_name).expect("read local source control state"),
        expected
    );
}
