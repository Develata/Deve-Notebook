//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Merge control WebSocket route.

use crate::server::handlers::merge;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

/// 路由手动合并模式相关消息。
pub(super) async fn route_merge(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    if let Some(scope_nonce) = requested_scope_nonce(&msg)
        && super::scope_guard::reject_invalid_browser_scope_nonce(
            ch,
            session,
            scope_nonce,
            "merge control",
        )
    {
        return;
    }
    match msg {
        ClientMessage::GetSyncMode { request_id, .. } => {
            merge::handle_get_sync_mode(state, ch, session, request_id).await;
        }
        ClientMessage::SetSyncMode { mode, .. } => {
            merge::handle_set_sync_mode(state, ch, session, mode).await;
        }
        ClientMessage::GetPendingOps { request_id, .. } => {
            merge::handle_get_pending_ops(state, ch, session, request_id).await;
        }
        ClientMessage::ConfirmMerge { .. } => {
            merge::handle_confirm_merge(state, ch, session).await;
        }
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action,
            result_content,
            ..
        } => {
            merge::handle_resolve_merge_conflict(
                state,
                ch,
                session,
                doc_id,
                action,
                result_content,
            )
            .await;
        }
        ClientMessage::DiscardPending { .. } => {
            merge::handle_discard_pending(state, ch, session).await;
        }
        ClientMessage::MergePeer {
            peer_id, doc_id, ..
        } => {
            merge::handle_merge_peer(state, ch, session, peer_id, doc_id).await;
        }
        other => super::source_control::route_source_control(state, ch, session, other).await,
    }
}

fn requested_scope_nonce(msg: &ClientMessage) -> Option<Option<u64>> {
    match msg {
        ClientMessage::GetSyncMode { scope_nonce, .. }
        | ClientMessage::SetSyncMode { scope_nonce, .. }
        | ClientMessage::GetPendingOps { scope_nonce, .. }
        | ClientMessage::ConfirmMerge { scope_nonce }
        | ClientMessage::ResolveMergeConflict { scope_nonce, .. }
        | ClientMessage::DiscardPending { scope_nonce }
        | ClientMessage::MergePeer { scope_nonce, .. } => Some(*scope_nonce),
        _ => None,
    }
}

#[cfg(test)]
#[path = "merge_readonly_test.rs"]
mod readonly_tests;

#[cfg(test)]
#[path = "merge_peer_test_support.rs"]
mod merge_peer_test_support;

#[cfg(test)]
#[path = "merge_peer_contract_test.rs"]
mod peer_contract_tests;

#[cfg(test)]
#[path = "merge_peer_resume_test.rs"]
mod peer_resume_tests;

#[cfg(test)]
mod tests {
    use super::{requested_scope_nonce, route_merge};
    use crate::server::session::PendingMergeConflict;
    use crate::server::sync_hello_test_support::{build_state, unicast_channel};
    use deve_core::models::{DocId, PeerId};
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
            assert_eq!(requested_scope_nonce(&msg), Some(Some(5)));
        }
        assert_eq!(requested_scope_nonce(&ClientMessage::Ping), None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn resolve_merge_conflict_routes_accept_current_to_merge_complete() -> anyhow::Result<()>
    {
        let (_dir, state, repo_id) = build_state()?;
        let (ch, _uni_rx) = unicast_channel(&state);
        let mut broadcast_rx = state.tx.subscribe();
        let doc_id = DocId::new();
        let mut session = crate::server::session::WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(17));
        session.pending_merge_conflict = Some(PendingMergeConflict {
            repo_id,
            repo_name: "notes".into(),
            branch: Some(PeerId::new("remote-a")),
            doc_id,
            scope_nonce: Some(17),
            local_content: "local".into(),
            incoming_content: "incoming".into(),
        });

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
    async fn resolve_merge_conflict_rejects_stale_scope_without_consuming_pending()
    -> anyhow::Result<()> {
        let (_dir, state, repo_id) = build_state()?;
        let (ch, mut uni_rx) = unicast_channel(&state);
        let doc_id = DocId::new();
        let mut session = crate::server::session::WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(17));
        session.pending_merge_conflict = Some(PendingMergeConflict {
            repo_id,
            repo_name: "notes".into(),
            branch: Some(PeerId::new("remote-a")),
            doc_id,
            scope_nonce: Some(17),
            local_content: "local".into(),
            incoming_content: "incoming".into(),
        });

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
}
