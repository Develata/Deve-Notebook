//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Source-control WebSocket route.

use crate::server::handlers::source_control;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

/// 路由版本控制相关消息。
pub(super) async fn route_source_control(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    if let Some(scope_nonce) = requested_scope_nonce(&msg)
        && let Err(error) =
            super::scope_guard::validate_browser_scope_nonce(session, scope_nonce, "source control")
    {
        ch.send_protocol_error_with_scope_nonce(
            error,
            super::scope_guard::response_scope_nonce(session, scope_nonce),
        );
        return;
    }
    match msg {
        ClientMessage::GetChanges { request_id, .. } => {
            source_control::handle_get_changes(state, ch, session, Some(request_id)).await;
        }
        ClientMessage::StageFile { target, .. } => {
            source_control::handle_stage_file(state, ch, session, target).await;
        }
        ClientMessage::StageFiles { targets, .. } => {
            source_control::handle_stage_files(state, ch, session, targets).await;
        }
        ClientMessage::UnstageFile { target, .. } => {
            source_control::handle_unstage_file(state, ch, session, target).await;
        }
        ClientMessage::UnstageFiles { targets, .. } => {
            source_control::handle_unstage_files(state, ch, session, targets).await;
        }
        ClientMessage::DiscardFile { target, .. } => {
            source_control::handle_discard_file(state, ch, session, target).await;
        }
        ClientMessage::Commit { message, .. } => {
            source_control::handle_commit(state, ch, session, message).await;
        }
        ClientMessage::GetCommitHistory {
            request_id, limit, ..
        } => {
            source_control::handle_get_commit_history(state, ch, session, request_id, limit).await;
        }
        ClientMessage::GetCommitDiff {
            request_id,
            commit_a,
            commit_b,
            ..
        } => {
            source_control::handle_get_commit_diff(
                state, ch, session, request_id, commit_a, commit_b,
            )
            .await;
        }
        ClientMessage::ResolveConflict {
            target, resolution, ..
        } => {
            source_control::handle_resolve_conflict(state, ch, session, target, resolution).await;
        }
        ClientMessage::CommitAndPush { message, .. } => {
            source_control::handle_commit_and_push(state, ch, session, message).await;
        }
        ClientMessage::GetDocDiff {
            request_id, target, ..
        } => {
            source_control::handle_get_doc_diff(state, ch, session, request_id, target).await;
        }
        other => super::core::route_core(state, ch, session, other).await,
    }
}

fn requested_scope_nonce(msg: &ClientMessage) -> Option<Option<u64>> {
    match msg {
        ClientMessage::GetChanges { scope_nonce, .. }
        | ClientMessage::StageFile { scope_nonce, .. }
        | ClientMessage::StageFiles { scope_nonce, .. }
        | ClientMessage::UnstageFile { scope_nonce, .. }
        | ClientMessage::UnstageFiles { scope_nonce, .. }
        | ClientMessage::DiscardFile { scope_nonce, .. }
        | ClientMessage::Commit { scope_nonce, .. }
        | ClientMessage::GetCommitHistory { scope_nonce, .. }
        | ClientMessage::GetDocDiff { scope_nonce, .. }
        | ClientMessage::GetCommitDiff { scope_nonce, .. }
        | ClientMessage::ResolveConflict { scope_nonce, .. }
        | ClientMessage::CommitAndPush { scope_nonce, .. } => Some(*scope_nonce),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{requested_scope_nonce, route_source_control};
    use crate::server::sync_hello_test_support::{build_state, unicast_channel};
    use deve_core::protocol::{ClientMessage, ScPathTarget, ServerErrorCode, ServerMessage};
    use deve_core::source_control::ConflictResolution;
    use tokio::time::{Duration, timeout};

    #[test]
    fn extracts_scope_nonce_from_source_control_messages() {
        let target = || ScPathTarget::from_path("notes/a.md");
        let cases = [
            ClientMessage::GetChanges {
                request_id: "req-1".into(),
                scope_nonce: Some(7),
            },
            ClientMessage::StageFile {
                target: target(),
                scope_nonce: Some(7),
            },
            ClientMessage::StageFiles {
                targets: vec![target()],
                scope_nonce: Some(7),
            },
            ClientMessage::UnstageFile {
                target: target(),
                scope_nonce: Some(7),
            },
            ClientMessage::UnstageFiles {
                targets: vec![target()],
                scope_nonce: Some(7),
            },
            ClientMessage::DiscardFile {
                target: target(),
                scope_nonce: Some(7),
            },
            ClientMessage::Commit {
                message: "msg".into(),
                scope_nonce: Some(7),
            },
            ClientMessage::GetCommitHistory {
                request_id: "req-2".into(),
                limit: 10,
                scope_nonce: Some(7),
            },
            ClientMessage::GetDocDiff {
                request_id: "req-3".into(),
                target: target(),
                scope_nonce: Some(7),
            },
            ClientMessage::GetCommitDiff {
                request_id: "req-4".into(),
                commit_a: None,
                commit_b: "head".into(),
                scope_nonce: Some(7),
            },
            ClientMessage::ResolveConflict {
                target: target(),
                resolution: ConflictResolution::KeepLedger,
                scope_nonce: Some(7),
            },
            ClientMessage::CommitAndPush {
                message: "msg".into(),
                scope_nonce: Some(7),
            },
        ];
        for msg in cases {
            assert_eq!(requested_scope_nonce(&msg), Some(Some(7)));
        }
        assert_eq!(requested_scope_nonce(&ClientMessage::Ping), None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn resolve_conflict_rejects_stale_scope_before_handler() -> anyhow::Result<()> {
        let (_dir, state, _repo_id) = build_state()?;
        let (ch, mut uni_rx) = unicast_channel(&state);
        let mut session = crate::server::session::WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(17));

        route_source_control(
            &state,
            &ch,
            &mut session,
            ClientMessage::ResolveConflict {
                target: ScPathTarget::from_path("notes/a.md"),
                resolution: ConflictResolution::KeepLedger,
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
                        .contains("source control scope nonce is stale")
                );
            }
            other => panic!("expected ProtocolError, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn source_control_scope_nonce_gate_rejects_missing_scope_before_handler()
    -> anyhow::Result<()> {
        let (_dir, state, _repo_id) = build_state()?;
        let (ch, mut uni_rx) = unicast_channel(&state);
        let mut session = browser_session(17);

        for msg in source_control_messages_with_scope(None) {
            route_source_control(&state, &ch, &mut session, msg).await;
            assert_scope_guard_error(
                recv_protocol_error(&mut uni_rx).await?,
                Some(17),
                "source control scope nonce missing",
            );
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn source_control_scope_nonce_gate_rejects_stale_scope_before_handler()
    -> anyhow::Result<()> {
        let (_dir, state, _repo_id) = build_state()?;
        let (ch, mut uni_rx) = unicast_channel(&state);
        let mut session = browser_session(17);

        for msg in source_control_messages_with_scope(Some(16)) {
            route_source_control(&state, &ch, &mut session, msg).await;
            assert_scope_guard_error(
                recv_protocol_error(&mut uni_rx).await?,
                Some(16),
                "source control scope nonce is stale",
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

    fn source_control_messages_with_scope(scope_nonce: Option<u64>) -> Vec<ClientMessage> {
        let target = ScPathTarget::from_path("notes/a.md");
        vec![
            ClientMessage::GetChanges {
                request_id: "changes".into(),
                scope_nonce,
            },
            ClientMessage::StageFile {
                target: target.clone(),
                scope_nonce,
            },
            ClientMessage::StageFiles {
                targets: vec![target.clone()],
                scope_nonce,
            },
            ClientMessage::UnstageFile {
                target: target.clone(),
                scope_nonce,
            },
            ClientMessage::UnstageFiles {
                targets: vec![target.clone()],
                scope_nonce,
            },
            ClientMessage::DiscardFile {
                target: target.clone(),
                scope_nonce,
            },
            ClientMessage::Commit {
                message: "msg".into(),
                scope_nonce,
            },
            ClientMessage::GetCommitHistory {
                request_id: "history".into(),
                limit: 10,
                scope_nonce,
            },
            ClientMessage::GetDocDiff {
                request_id: "doc-diff".into(),
                target: target.clone(),
                scope_nonce,
            },
            ClientMessage::GetCommitDiff {
                request_id: "commit-diff".into(),
                commit_a: None,
                commit_b: "head".into(),
                scope_nonce,
            },
            ClientMessage::ResolveConflict {
                target,
                resolution: ConflictResolution::KeepLedger,
                scope_nonce,
            },
            ClientMessage::CommitAndPush {
                message: "msg".into(),
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
