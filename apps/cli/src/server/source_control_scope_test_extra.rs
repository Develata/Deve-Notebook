//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Source-control remote scope tests.

use super::support::recv_changes;
use super::*;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{DocId, LedgerEntry, NodeId, Op, PeerId, RepoId, StructureOp};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::source_control::{ChangeStatus, CommitInfo};
use deve_core::source_control::commits::{self, COMMITS_ORDER_TABLE};

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

fn shadow_create_file(peer_id: &PeerId, doc_id: DocId, name: &str, timestamp: i64) -> LedgerEntry {
    LedgerEntry::new_structure(
        StructureOp::CreateFile {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id,
            parent_id: None,
            name: name.into(),
        },
        timestamp,
        peer_id.clone(),
        timestamp as u64,
    )
}

fn shadow_insert(
    peer_id: &PeerId,
    doc_id: DocId,
    pos: u32,
    content: &str,
    timestamp: i64,
) -> LedgerEntry {
    LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos,
            content: content.into(),
        },
        timestamp,
        peer_id.clone(),
        timestamp as u64,
        None,
        None,
    )
}

fn shadow_rename(peer_id: &PeerId, doc_id: DocId, new_name: &str, timestamp: i64) -> LedgerEntry {
    LedgerEntry::new_structure(
        StructureOp::RenameNode {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id: Some(doc_id),
            new_name: new_name.into(),
        },
        timestamp,
        peer_id.clone(),
        timestamp as u64,
    )
}

fn create_shadow_commit(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: &RepoId,
    message: &str,
    ledger_seq: u64,
) -> anyhow::Result<CommitInfo> {
    repo.run_on_shadow_repo_by_id(peer_id, repo_id, |db| {
        commits::create(db, message, 1, ledger_seq)
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_doc_diff_uses_shadow_projection() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let doc_id = DocId::new();
    state.repo.append_remote_ops(
        &peer_id,
        &test_id,
        &[
            shadow_create_file(&peer_id, doc_id, "note.md", 1),
            shadow_insert(&peer_id, doc_id, 0, "hello", 2),
            shadow_insert(&peer_id, doc_id, 5, " remote", 3),
        ],
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(23));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "doc-req-1".into(),
        ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::DocDiff {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            doc_id: actual_doc_id,
            path,
            old_content,
            new_content,
        }) => {
            assert_eq!(request_id.as_deref(), Some("doc-req-1"));
            assert_eq!(repo_id, Some(test_id));
            assert_eq!(branch, Some(peer_id));
            assert_eq!(scope_nonce, Some(23));
            assert_eq!(actual_doc_id, Some(doc_id));
            assert_eq!(path, "note.md");
            assert_eq!(old_content, "");
            assert_eq!(new_content, "hello remote");
        }
        other => panic!("expected DocDiff, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_doc_diff_missing_target_returns_scoped_error() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(29));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "doc-req-missing".into(),
        ScPathTarget {
            path: "missing.md".into(),
            doc_id: None,
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScDocNotFound);
            assert_eq!(
                error.detail.as_deref(),
                Some("Remote document not found: missing.md")
            );
            assert_eq!(scope_nonce, Some(29));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_doc_diff_path_mismatch_returns_scoped_error() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let doc_id = DocId::new();
    state.repo.append_remote_ops(
        &peer_id,
        &test_id,
        &[
            shadow_create_file(&peer_id, doc_id, "note.md", 1),
            shadow_insert(&peer_id, doc_id, 0, "hello", 2),
        ],
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(31));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "doc-req-mismatch".into(),
        ScPathTarget {
            path: "other.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert!(error.detail.as_deref().is_some_and(|detail| {
                detail.contains("Remote document target path mismatch")
                    && detail.contains("requested other.md")
                    && detail.contains("is at note.md")
            }));
            assert_eq!(scope_nonce, Some(31));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_commit_history_is_allowed() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_commit_history(&state, &ch, &mut session, "req-1".into(), 10).await;
    let (repo_id, first_message) = recv_history(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(first_message, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_commit_diff_is_allowed() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let doc_id = DocId::new();
    state
        .repo
        .append_remote_op(&peer_id, &test_id, &shadow_create_file(&peer_id, doc_id, "note.md", 1))?;
    let first_ledger_seq = state
        .repo
        .append_remote_op(&peer_id, &test_id, &shadow_insert(&peer_id, doc_id, 0, "hello", 2))?;
    let first = create_shadow_commit(state.repo.as_ref(), &peer_id, &test_id, "first", first_ledger_seq)?;
    let second_ledger_seq = state
        .repo
        .append_remote_op(&peer_id, &test_id, &shadow_insert(&peer_id, doc_id, 5, " remote", 3))?;
    let second =
        create_shadow_commit(state.repo.as_ref(), &peer_id, &test_id, "second", second_ledger_seq)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(17));

    handle_get_commit_diff(
        &state,
        &ch,
        &mut session,
        "req-1".into(),
        Some(first.id),
        second.id,
    )
    .await;
    let (repo_id, branch, scope_nonce, diffs) = recv_commit_diff(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(branch, Some(peer_id));
    assert_eq!(scope_nonce, Some(17));
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, Some(doc_id));
    assert_eq!(diffs[0].path, "note.md");
    assert_eq!(diffs[0].status, ChangeStatus::Modified);
    assert_eq!(diffs[0].old_content, "hello");
    assert_eq!(diffs[0].new_content, "hello remote");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_commit_diff_reports_rename_projection() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let doc_id = DocId::new();
    state
        .repo
        .append_remote_op(&peer_id, &test_id, &shadow_create_file(&peer_id, doc_id, "note.md", 1))?;
    let first_ledger_seq = state
        .repo
        .append_remote_op(&peer_id, &test_id, &shadow_insert(&peer_id, doc_id, 0, "hello", 2))?;
    let first = create_shadow_commit(state.repo.as_ref(), &peer_id, &test_id, "first", first_ledger_seq)?;
    let second_ledger_seq = state
        .repo
        .append_remote_op(&peer_id, &test_id, &shadow_rename(&peer_id, doc_id, "renamed.md", 3))?;
    let second =
        create_shadow_commit(state.repo.as_ref(), &peer_id, &test_id, "rename", second_ledger_seq)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(19));

    handle_get_commit_diff(
        &state,
        &ch,
        &mut session,
        "req-rename".into(),
        Some(first.id),
        second.id,
    )
    .await;
    let (repo_id, branch, scope_nonce, diffs) = recv_commit_diff(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(branch, Some(peer_id));
    assert_eq!(scope_nonce, Some(19));
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, Some(doc_id));
    assert_eq!(diffs[0].previous_path.as_deref(), Some("note.md"));
    assert_eq!(diffs[0].path, "renamed.md");
    assert_eq!(diffs[0].status, ChangeStatus::Renamed);
    assert_eq!(diffs[0].old_content, "hello");
    assert_eq!(diffs[0].new_content, "hello");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_history_repairs_legacy_missing_order_table() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    state
        .repo
        .run_on_shadow_repo_by_id(&peer_id, &test_id, |db| {
            let _first = commits::create(db, "first", 1, 1)?;
            let _second = commits::create(db, "second", 1, 2)?;
            let write_txn = db.begin_write()?;
            let _ = write_txn.delete_table(COMMITS_ORDER_TABLE)?;
            write_txn.commit()?;
            Ok::<(), anyhow::Error>(())
        })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_commit_history(&state, &ch, &mut session, "req-1".into(), 10).await;
    let (repo_id, first_message) = recv_history(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(first_message.as_deref(), Some("second"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_changes_are_allowed_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;
    let (repo_id, paths) = recv_changes(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert!(paths.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_changes_without_repo_selection_clear_stale_db_and_sync_binding()
-> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("peer-a".into()));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(13);

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Remote branch not available:"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
