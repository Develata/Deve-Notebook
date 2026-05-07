use super::route_merge;
use crate::server::session::PendingMergeConflict;
use crate::server::sync_hello_test_support::{build_state, unicast_channel};
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::{ClientMessage, MergeConflictAction, ServerErrorCode, ServerMessage};
use tokio::time::{Duration, timeout};

#[test]
fn extracts_scope_nonce_from_merge_messages() {
    let doc_id = DocId::new();
    let cases = [
        ClientMessage::GetSyncMode {
            request_id: "req-1".into(),
            scope_nonce: Some(5),
        },
        ClientMessage::SetSyncMode {
            mode: "manual".into(),
            scope_nonce: Some(5),
        },
        ClientMessage::GetPendingOps {
            request_id: "req-2".into(),
            scope_nonce: Some(5),
        },
        ClientMessage::ConfirmMerge {
            scope_nonce: Some(5),
        },
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action: MergeConflictAction::AcceptIncoming,
            result_content: None,
            scope_nonce: Some(5),
        },
        ClientMessage::DiscardPending {
            scope_nonce: Some(5),
        },
        ClientMessage::MergePeer {
            peer_id: "remote-a".into(),
            doc_id,
            scope_nonce: Some(5),
        },
    ];
    for msg in cases {
        let gate = msg.merge_control_scope_gate().expect("scope gate");
        assert_eq!(gate.scope_nonce, Some(5));
        assert_eq!(gate.scope_name, "merge control");
    }
    assert_eq!(ClientMessage::Ping.merge_control_scope_gate(), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_merge_conflict_routes_accept_current_to_merge_complete() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, _uni_rx) = unicast_channel(&state);
    let mut broadcast_rx = state.tx.subscribe();
    let doc_id = DocId::new();
    let mut session = browser_session_with_pending_conflict(repo_id, doc_id, 17);

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action: MergeConflictAction::AcceptCurrent,
            result_content: None,
            scope_nonce: Some(17),
        },
    )
    .await;

    match timeout(Duration::from_secs(2), broadcast_rx.recv()).await?? {
        ServerMessage::MergeComplete {
            repo_id: actual_repo,
            branch,
            scope_nonce,
            merged_count,
        } => {
            assert_eq!(actual_repo, Some(repo_id));
            assert_eq!(branch, Some(PeerId::new("remote-a")));
            assert_eq!(scope_nonce, Some(17));
            assert_eq!(merged_count, 0);
        }
        other => panic!("expected MergeComplete, got {other:?}"),
    }
    assert!(session.pending_merge_conflict.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_merge_conflict_rejects_stale_scope_without_consuming_pending() -> anyhow::Result<()>
{
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let doc_id = DocId::new();
    let mut session = browser_session_with_pending_conflict(repo_id, doc_id, 17);

    route_merge(
        &state,
        &ch,
        &mut session,
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action: MergeConflictAction::AcceptIncoming,
            result_content: None,
            scope_nonce: Some(16),
        },
    )
    .await;

    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("protocol error")
    {
        ServerMessage::ProtocolError {
            error, scope_nonce, ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(16));
            assert!(
                error
                    .detail
                    .as_deref()
                    .expect("detail")
                    .contains("merge control scope nonce is stale")
            );
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
    let pending = session
        .pending_merge_conflict
        .as_ref()
        .expect("pending conflict should remain");
    assert_eq!(pending.doc_id, doc_id);
    assert_eq!(pending.scope_nonce, Some(17));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_scope_nonce_gate_rejects_missing_scope_before_handler() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    for msg in merge_messages_with_scope(None) {
        route_merge(&state, &ch, &mut session, msg).await;
        assert_scope_guard_error(
            recv_protocol_error(&mut uni_rx).await?,
            Some(17),
            "merge control scope nonce missing",
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_scope_nonce_gate_rejects_stale_scope_before_handler() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    for msg in merge_messages_with_scope(Some(16)) {
        route_merge(&state, &ch, &mut session, msg).await;
        assert_scope_guard_error(
            recv_protocol_error(&mut uni_rx).await?,
            Some(16),
            "merge control scope nonce is stale",
        );
    }
    Ok(())
}

fn browser_session(scope_nonce: u64) -> crate::server::session::WsSession {
    let mut session = crate::server::session::WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

fn browser_session_with_pending_conflict(
    repo_id: RepoId,
    doc_id: DocId,
    scope_nonce: u64,
) -> crate::server::session::WsSession {
    let mut session = browser_session(scope_nonce);
    session.pending_merge_conflict = Some(PendingMergeConflict {
        repo_id,
        repo_name: "notes".into(),
        branch: Some(PeerId::new("remote-a")),
        doc_id,
        scope_nonce: Some(scope_nonce),
        local_content: "local".into(),
        incoming_content: "incoming".into(),
    });
    session
}

fn merge_messages_with_scope(scope_nonce: Option<u64>) -> Vec<ClientMessage> {
    let doc_id = DocId::new();
    vec![
        ClientMessage::GetSyncMode {
            request_id: "sync-mode".into(),
            scope_nonce,
        },
        ClientMessage::SetSyncMode {
            mode: "manual".into(),
            scope_nonce,
        },
        ClientMessage::GetPendingOps {
            request_id: "pending".into(),
            scope_nonce,
        },
        ClientMessage::ConfirmMerge { scope_nonce },
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action: MergeConflictAction::AcceptIncoming,
            result_content: None,
            scope_nonce,
        },
        ClientMessage::DiscardPending { scope_nonce },
        ClientMessage::MergePeer {
            peer_id: "remote-a".into(),
            doc_id,
            scope_nonce,
        },
    ]
}

async fn recv_protocol_error(
    uni_rx: &mut tokio::sync::mpsc::Receiver<ServerMessage>,
) -> anyhow::Result<ServerMessage> {
    Ok(timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("protocol error"))
}

fn assert_scope_guard_error(message: ServerMessage, scope_nonce: Option<u64>, detail: &str) {
    match message {
        ServerMessage::ProtocolError {
            error,
            scope_nonce: actual_scope_nonce,
            ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(actual_scope_nonce, scope_nonce);
            assert!(error.detail.as_deref().expect("detail").contains(detail));
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
}
