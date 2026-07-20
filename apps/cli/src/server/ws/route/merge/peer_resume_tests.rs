//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 05_diff_logic#merge-contract
//!
//! Merge peer interruption and deterministic replay tests.

use super::merge_peer_test_support::{
    MergeConflictExpectation, browser_writer_ready_session, doc_content, doc_entry_count,
    ensure_local_projection_ready, ensure_remote_repo, expect_merge_complete,
    expect_merge_conflict, local_doc_content, local_doc_entry_count, reopen_state,
    request_merge_peer, seed_local_doc, seed_local_replace, seed_remote_replace, seed_shared_base,
};
use super::route_merge;
use crate::server::sync_hello_test_support::{build_state, unicast_channel};
use deve_core::models::RepoType;
use deve_core::protocol::{ClientMessage, MergeConflictAction};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_peer_conflict_replays_after_state_reopen_without_losing_ops() -> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    ensure_local_projection_ready(&state)?;
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let doc_id = seed_local_doc(&state, "resume.md")?;
    seed_shared_base(&state, &peer_id, repo_id, doc_id, "base")?;
    seed_local_replace(&state, doc_id, "base", "local")?;
    seed_remote_replace(&state, &peer_id, repo_id, doc_id, "base", "remote")?;
    let local_before = local_doc_content(&state, doc_id)?;
    let remote_before = doc_content(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    let local_entries_before = local_doc_entry_count(&state, doc_id)?;
    let remote_entries_before =
        doc_entry_count(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;

    let (first_ch, mut first_rx) = unicast_channel(&state);
    let mut interrupted = browser_writer_ready_session(repo_id, 61);
    request_merge_peer(&state, &first_ch, &mut interrupted, &peer_id, doc_id, 61).await;
    expect_merge_conflict(
        &mut first_rx,
        MergeConflictExpectation {
            repo_id,
            branch: None,
            scope_nonce: Some(61),
            doc_id,
            path: "resume.md",
            current_content: "local",
            incoming_content: "remote",
            result_content: "local\nremote",
            start_line: 0,
            length: 1,
            local_lines: &["local"],
            remote_lines: &["remote"],
        },
    )
    .await?;
    assert!(interrupted.pending_merge_conflict.is_some());
    drop(interrupted);
    drop(first_rx);
    drop(first_ch);
    drop(state);

    let state = reopen_state(dir.path(), repo_id)?;
    assert_eq!(
        state.repo.get_repo_info()?.expect("repo info").uuid,
        repo_id
    );
    assert_eq!(local_doc_entry_count(&state, doc_id)?, local_entries_before);
    assert_eq!(
        doc_entry_count(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?,
        remote_entries_before
    );
    let (resume_ch, mut resume_rx) = unicast_channel(&state);
    let mut broadcast_rx = state.tx.subscribe();
    let mut resumed = browser_writer_ready_session(repo_id, 62);
    request_merge_peer(&state, &resume_ch, &mut resumed, &peer_id, doc_id, 62).await;
    expect_merge_conflict(
        &mut resume_rx,
        MergeConflictExpectation {
            repo_id,
            branch: None,
            scope_nonce: Some(62),
            doc_id,
            path: "resume.md",
            current_content: "local",
            incoming_content: "remote",
            result_content: "local\nremote",
            start_line: 0,
            length: 1,
            local_lines: &["local"],
            remote_lines: &["remote"],
        },
    )
    .await?;
    assert_eq!(local_doc_content(&state, doc_id)?, local_before);
    assert_eq!(
        doc_content(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?,
        remote_before
    );

    route_merge(
        &state,
        &resume_ch,
        &mut resumed,
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action: MergeConflictAction::AcceptIncoming,
            result_content: None,
            scope_nonce: Some(62),
        },
    )
    .await;

    expect_merge_complete(&mut broadcast_rx, repo_id, None, Some(62), 1).await?;
    assert!(resumed.pending_merge_conflict.is_none());
    assert_eq!(local_doc_content(&state, doc_id)?.1, "remote");
    assert!(local_doc_entry_count(&state, doc_id)? > local_entries_before);
    assert_eq!(
        doc_content(&state, RepoType::Remote(peer_id, repo_id), doc_id)?,
        remote_before
    );
    Ok(())
}
