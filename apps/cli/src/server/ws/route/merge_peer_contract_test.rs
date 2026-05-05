//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 07_diff_logic#merge-contract
//!
//! Merge peer local-branch authority contract tests.

use super::route_merge;
use crate::server::session::WsSession;
use crate::server::sync_hello_test_support::{build_state, unicast_channel};
use deve_core::ledger::{RepoInfo, node_meta};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId, RepoType};
use deve_core::protocol::{ClientMessage, MergeConflictAction, ServerErrorCode, ServerMessage};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, timeout};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_peer_local_branch_contract_writes_local_only() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let doc_id = seed_local_doc(&state, "notes/a.md")?;
    seed_remote_insert(&state, &peer_id, repo_id, doc_id, "incoming")?;
    let remote_before = doc_content(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    let local_before = local_doc_content(&state, doc_id)?;
    let (ch, _uni_rx) = unicast_channel(&state);
    let mut broadcast_rx = state.tx.subscribe();
    let mut session = browser_remote_session(&peer_id, repo_id, 41);

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
    assert_eq!(local_before, (0, String::new()));
    assert_eq!(local_doc_content(&state, doc_id)?.1, "incoming");
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
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let doc_id = seed_local_doc(&state, "notes/conflict.md")?;
    seed_shared_base(&state, &peer_id, repo_id, doc_id, "base")?;
    seed_local_replace(&state, doc_id, "base", "local")?;
    seed_remote_replace(&state, &peer_id, repo_id, doc_id, "base", "remote")?;
    let remote_before = doc_content(&state, RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    let local_before = local_doc_content(&state, doc_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut broadcast_rx = state.tx.subscribe();
    let mut session = browser_remote_session(&peer_id, repo_id, case.scope_nonce);

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

fn ensure_remote_repo(
    state: &std::sync::Arc<crate::server::AppState>,
    repo_id: uuid::Uuid,
) -> anyhow::Result<PeerId> {
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "notes".into(),
            url: Some("urn:test:notes".into()),
        },
    )?;
    Ok(peer_id)
}

fn seed_local_doc(
    state: &std::sync::Arc<crate::server::AppState>,
    path: &str,
) -> anyhow::Result<DocId> {
    let doc_id = DocId::new();
    state.repo.run_on_local_repo("notes", |db| {
        node_meta::ensure_file_node(db, path, doc_id)?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(doc_id)
}

fn seed_remote_insert(
    state: &std::sync::Arc<crate::server::AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    content: &str,
) -> anyhow::Result<()> {
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
            peer_id.clone(),
            1,
            None,
            None,
        ),
    )?;
    Ok(())
}

fn seed_shared_base(
    state: &std::sync::Arc<crate::server::AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    content: &str,
) -> anyhow::Result<()> {
    let base_entry = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 0,
            content: content.into(),
        },
        1,
        PeerId::new("shared-base"),
        1,
        None,
        None,
    );
    state
        .repo
        .append_local_op_in_local_repo("notes", &base_entry)?;
    state
        .repo
        .append_remote_op(peer_id, &repo_id, &base_entry)?;
    Ok(())
}

fn seed_local_replace(
    state: &std::sync::Arc<crate::server::AppState>,
    doc_id: DocId,
    before: &str,
    after: &str,
) -> anyhow::Result<()> {
    let peer_id = PeerId::new("local-test");
    state.repo.append_local_op_in_local_repo(
        "notes",
        &LedgerEntry::new_content(
            doc_id,
            Op::Delete {
                pos: 0,
                len: utf16_len(before),
            },
            2,
            peer_id.clone(),
            1,
            None,
            None,
        ),
    )?;
    state.repo.append_local_op_in_local_repo(
        "notes",
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: after.into(),
            },
            3,
            peer_id,
            2,
            None,
            None,
        ),
    )?;
    Ok(())
}

fn seed_remote_replace(
    state: &std::sync::Arc<crate::server::AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    before: &str,
    after: &str,
) -> anyhow::Result<()> {
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Delete {
                pos: 0,
                len: utf16_len(before),
            },
            2,
            peer_id.clone(),
            1,
            None,
            None,
        ),
    )?;
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: after.into(),
            },
            3,
            peer_id.clone(),
            2,
            None,
            None,
        ),
    )?;
    Ok(())
}

fn utf16_len(value: &str) -> u32 {
    value.encode_utf16().count() as u32
}

fn browser_remote_session(peer_id: &PeerId, repo_id: uuid::Uuid, scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(scope_nonce));
    session
}

fn local_doc_content(
    state: &std::sync::Arc<crate::server::AppState>,
    doc_id: DocId,
) -> anyhow::Result<(usize, String)> {
    let entries = state
        .repo
        .get_local_ops_in_local_repo("notes", doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    Ok((
        entries.len(),
        deve_core::state::reconstruct_content(&entries),
    ))
}

fn doc_content(
    state: &std::sync::Arc<crate::server::AppState>,
    repo_type: RepoType,
    doc_id: DocId,
) -> anyhow::Result<(usize, String)> {
    let entries = state
        .repo
        .get_ops(&repo_type, doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    Ok((
        entries.len(),
        deve_core::state::reconstruct_content(&entries),
    ))
}

async fn expect_merge_complete(
    broadcast_rx: &mut broadcast::Receiver<ServerMessage>,
    repo_id: uuid::Uuid,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    merged_count: u32,
) -> anyhow::Result<()> {
    match timeout(Duration::from_secs(2), broadcast_rx.recv()).await?? {
        ServerMessage::MergeComplete {
            repo_id: actual_repo,
            branch: actual_branch,
            scope_nonce: actual_scope_nonce,
            merged_count: actual_count,
        } => {
            assert_eq!(actual_repo, Some(repo_id));
            assert_eq!(actual_branch, branch);
            assert_eq!(actual_scope_nonce, scope_nonce);
            assert_eq!(actual_count, merged_count);
        }
        other => panic!("expected MergeComplete, got {other:?}"),
    }
    Ok(())
}

struct MergeConflictExpectation<'a> {
    repo_id: uuid::Uuid,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    path: &'a str,
    current_content: &'a str,
    incoming_content: &'a str,
    result_content: &'a str,
    start_line: usize,
    length: usize,
    local_lines: &'a [&'a str],
    remote_lines: &'a [&'a str],
}

async fn expect_merge_conflict(
    uni_rx: &mut mpsc::Receiver<ServerMessage>,
    expected: MergeConflictExpectation<'_>,
) -> anyhow::Result<()> {
    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("merge conflict")
    {
        ServerMessage::MergeConflict {
            repo_id: actual_repo,
            branch: actual_branch,
            scope_nonce: actual_scope_nonce,
            doc_id: actual_doc_id,
            path,
            current_content,
            incoming_content,
            result_content,
            actions,
            conflicts,
        } => {
            assert_eq!(actual_repo, Some(expected.repo_id));
            assert_eq!(actual_branch, expected.branch);
            assert_eq!(actual_scope_nonce, expected.scope_nonce);
            assert_eq!(actual_doc_id, expected.doc_id);
            assert_eq!(path, expected.path);
            assert_eq!(current_content, expected.current_content);
            assert_eq!(incoming_content, expected.incoming_content);
            assert_eq!(result_content, expected.result_content);
            assert_eq!(actions.len(), 3);
            assert!(actions.contains(&MergeConflictAction::AcceptCurrent));
            assert!(actions.contains(&MergeConflictAction::AcceptIncoming));
            assert!(actions.contains(&MergeConflictAction::AcceptBoth));
            assert_eq!(conflicts.len(), 1);
            let conflict = &conflicts[0];
            assert_eq!(conflict.start_line, expected.start_line);
            assert_eq!(conflict.length, expected.length);
            assert_eq!(
                conflict.local_lines,
                expected
                    .local_lines
                    .iter()
                    .map(|line| (*line).to_string())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                conflict.remote_lines,
                expected
                    .remote_lines
                    .iter()
                    .map(|line| (*line).to_string())
                    .collect::<Vec<_>>()
            );
        }
        other => panic!("expected MergeConflict, got {other:?}"),
    }
    Ok(())
}

fn drain_unicast(uni_rx: &mut mpsc::Receiver<ServerMessage>) {
    while let Ok(message) = uni_rx.try_recv() {
        if let ServerMessage::ProtocolError { error, .. } = message {
            assert_eq!(error.code, ServerErrorCode::StorageConflict);
        }
    }
}
