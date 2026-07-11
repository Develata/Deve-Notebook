//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 05_diff_logic#merge-contract
//!
//! Merge peer local-branch authority contract tests.

use super::merge_peer_test_support::{
    MergeConflictExpectation, browser_local_session, browser_remote_session,
    browser_writer_ready_session, doc_content, ensure_local_projection_ready, ensure_remote_repo,
    expect_merge_complete, expect_merge_conflict, local_doc_content, seed_local_doc,
    seed_local_replace, seed_remote_insert, seed_remote_replace, seed_shared_base,
};
use super::route_merge;
use crate::server::session::PendingMergeConflict;
use crate::server::sync_hello_test_support::{build_state, unicast_channel};
use deve_core::models::RepoType;
use deve_core::protocol::{ClientMessage, MergeConflictAction, ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_peer_local_branch_contract_writes_local_only() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    ensure_local_projection_ready(&state)?;
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let doc_id = seed_local_doc(&state, "notes/a.md")?;
    seed_shared_base(&state, &peer_id, repo_id, doc_id, "base")?;
    seed_remote_replace(&state, &peer_id, repo_id, doc_id, "base", "incoming")?;
    let remote_before = doc_content(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    let local_before = local_doc_content(&state, doc_id)?;
    let (ch, _uni_rx) = unicast_channel(&state);
    let mut broadcast_rx = state.tx.subscribe();
    let mut session = browser_writer_ready_session(repo_id, 41);

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::MergePeer {
            peer_id: peer_id.to_string(),
            doc_id,
            scope_nonce: Some(41),
        },
    )
    .await;

    expect_merge_complete(&mut broadcast_rx, repo_id, None, Some(41), 1).await?;
    assert_eq!(local_before.1, "base");
    assert_eq!(local_doc_content(&state, doc_id)?.1, "incoming");
    assert_eq!(
        doc_content(&state, RepoType::Remote(peer_id, repo_id), doc_id)?,
        remote_before
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_peer_rejects_local_branch_without_writer_ready() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    ensure_local_projection_ready(&state)?;
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let doc_id = seed_local_doc(&state, "notes/local-writer-reject.md")?;
    seed_remote_insert(&state, &peer_id, repo_id, doc_id, "incoming")?;
    let remote_before = doc_content(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    let local_before = local_doc_content(&state, doc_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_local_session(repo_id, 45);

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::MergePeer {
            peer_id: peer_id.to_string(),
            doc_id,
            scope_nonce: Some(45),
        },
    )
    .await;

    expect_protocol_error(&mut uni_rx, ServerErrorCode::SyncPeerUnauthenticated, 45).await?;
    assert_eq!(local_doc_content(&state, doc_id)?, local_before);
    assert_eq!(
        doc_content(&state, RepoType::Remote(peer_id, repo_id), doc_id)?,
        remote_before
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_peer_rejects_remote_branch_scope() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let doc_id = seed_local_doc(&state, "notes/remote-reject.md")?;
    seed_remote_insert(&state, &peer_id, repo_id, doc_id, "incoming")?;
    let remote_before = doc_content(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    let local_before = local_doc_content(&state, doc_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_remote_session(&peer_id, repo_id, 49);

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::MergePeer {
            peer_id: peer_id.to_string(),
            doc_id,
            scope_nonce: Some(49),
        },
    )
    .await;

    expect_remote_readonly_error(&mut uni_rx, 49).await?;
    assert_eq!(local_doc_content(&state, doc_id)?, local_before);
    assert_eq!(
        doc_content(&state, RepoType::Remote(peer_id, repo_id), doc_id)?,
        remote_before
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_merge_conflict_local_branch_contract_writes_local_only() -> anyhow::Result<()> {
    assert_resolve_merge_conflict_strategy(ResolveStrategyCase {
        action: MergeConflictAction::AcceptIncoming,
        result_content: None,
        expected_content: "remote",
        expected_merged_count: 1,
        scope_nonce: 43,
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_merge_conflict_rejects_remote_branch_scope_without_consuming_pending()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let doc_id = seed_local_doc(&state, "notes/remote-resolve-reject.md")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_remote_session(&peer_id, repo_id, 57);
    session.pending_merge_conflict = Some(PendingMergeConflict {
        repo_id,
        branch: None,
        doc_id,
        scope_nonce: Some(57),
        local_content: "local".into(),
        incoming_content: "incoming".into(),
        preflight: crate::server::session::test_merge_preflight(
            repo_id, doc_id, "local", "incoming",
        ),
    });

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action: MergeConflictAction::AcceptIncoming,
            result_content: None,
            scope_nonce: Some(57),
        },
    )
    .await;

    expect_remote_readonly_error(&mut uni_rx, 57).await?;
    assert!(session.pending_merge_conflict.is_some());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_merge_conflict_rejects_local_branch_without_writer_ready_without_consuming_pending()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    ensure_local_projection_ready(&state)?;
    let doc_id = seed_local_doc(&state, "notes/local-resolve-writer-reject.md")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_local_session(repo_id, 59);
    session.pending_merge_conflict = Some(PendingMergeConflict {
        repo_id,
        branch: None,
        doc_id,
        scope_nonce: Some(59),
        local_content: "local".into(),
        incoming_content: "incoming".into(),
        preflight: crate::server::session::test_merge_preflight(
            repo_id, doc_id, "local", "incoming",
        ),
    });

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action: MergeConflictAction::AcceptIncoming,
            result_content: None,
            scope_nonce: Some(59),
        },
    )
    .await;

    expect_protocol_error(&mut uni_rx, ServerErrorCode::SyncPeerUnauthenticated, 59).await?;
    assert!(session.pending_merge_conflict.is_some());
    assert_eq!(local_doc_content(&state, doc_id)?.1, "");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_merge_conflict_accept_current_keeps_local_branch_without_write()
-> anyhow::Result<()> {
    assert_resolve_merge_conflict_strategy(ResolveStrategyCase {
        action: MergeConflictAction::AcceptCurrent,
        result_content: None,
        expected_content: "local",
        expected_merged_count: 0,
        scope_nonce: 47,
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_merge_conflict_accept_both_writes_result_to_local_branch() -> anyhow::Result<()> {
    assert_resolve_merge_conflict_strategy(ResolveStrategyCase {
        action: MergeConflictAction::AcceptBoth,
        result_content: Some("local\nremote\nmanual".into()),
        expected_content: "local\nremote\nmanual",
        expected_merged_count: 1,
        scope_nonce: 53,
    })
    .await
}

struct ResolveStrategyCase {
    action: MergeConflictAction,
    result_content: Option<String>,
    expected_content: &'static str,
    expected_merged_count: u32,
    scope_nonce: u64,
}

async fn assert_resolve_merge_conflict_strategy(case: ResolveStrategyCase) -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    ensure_local_projection_ready(&state)?;
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let doc_id = seed_local_doc(&state, "notes/conflict.md")?;
    seed_shared_base(&state, &peer_id, repo_id, doc_id, "base")?;
    seed_local_replace(&state, doc_id, "base", "local")?;
    seed_remote_replace(&state, &peer_id, repo_id, doc_id, "base", "remote")?;
    let remote_before = doc_content(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    let local_before = local_doc_content(&state, doc_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut broadcast_rx = state.tx.subscribe();
    let mut session = browser_writer_ready_session(repo_id, case.scope_nonce);

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::MergePeer {
            peer_id: peer_id.to_string(),
            doc_id,
            scope_nonce: Some(case.scope_nonce),
        },
    )
    .await;
    expect_merge_conflict(
        &mut uni_rx,
        MergeConflictExpectation {
            repo_id,
            branch: None,
            scope_nonce: Some(case.scope_nonce),
            doc_id,
            path: "notes/conflict.md",
            current_content: "local",
            incoming_content: "remote",
            result_content: "base",
            start_line: 0,
            length: 1,
            local_lines: &["local"],
            remote_lines: &["remote"],
        },
    )
    .await?;
    drain_unicast(&mut uni_rx);
    assert_eq!(
        session
            .pending_merge_conflict
            .as_ref()
            .map(|pending| pending.branch.clone()),
        Some(None)
    );

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action: case.action,
            result_content: case.result_content,
            scope_nonce: Some(case.scope_nonce),
        },
    )
    .await;

    expect_merge_complete(
        &mut broadcast_rx,
        repo_id,
        None,
        Some(case.scope_nonce),
        case.expected_merged_count,
    )
    .await?;
    assert!(session.pending_merge_conflict.is_none());
    let local_after = local_doc_content(&state, doc_id)?;
    assert_eq!(local_after.1, case.expected_content);
    if case.expected_merged_count == 0 {
        assert_eq!(local_after, local_before);
    } else {
        assert!(
            local_after.0 > local_before.0,
            "resolved merge should append a local op"
        );
    }
    assert_eq!(
        doc_content(&state, RepoType::Remote(peer_id, repo_id), doc_id)?,
        remote_before
    );
    Ok(())
}

fn drain_unicast(uni_rx: &mut mpsc::Receiver<ServerMessage>) {
    while let Ok(message) = uni_rx.try_recv() {
        if let ServerMessage::ProtocolError { error, .. } = message {
            assert_eq!(error.code, ServerErrorCode::StorageConflict);
        }
    }
}

async fn expect_remote_readonly_error(
    uni_rx: &mut mpsc::Receiver<ServerMessage>,
    scope_nonce: u64,
) -> anyhow::Result<()> {
    expect_protocol_error(uni_rx, ServerErrorCode::ScRemoteBranchReadonly, scope_nonce).await
}

async fn expect_protocol_error(
    uni_rx: &mut mpsc::Receiver<ServerMessage>,
    expected_code: ServerErrorCode,
    scope_nonce: u64,
) -> anyhow::Result<()> {
    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("protocol error")
    {
        ServerMessage::ProtocolError {
            error,
            scope_nonce: actual_scope_nonce,
            ..
        } => {
            assert_eq!(error.code, expected_code);
            assert_eq!(actual_scope_nonce, Some(scope_nonce));
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
    Ok(())
}
