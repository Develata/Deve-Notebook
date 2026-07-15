//! plan_ref:
//!   - 07_network#server-ws-runtime
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
    if let Some(scope) = msg.source_control_scope_gate()
        && super::scope_guard::reject_invalid_browser_scope_nonce(
            ch,
            session,
            scope.scope_nonce,
            scope.scope_name,
        )
    {
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
        ClientMessage::ApplyExternalChanges { request_id, .. } => {
            source_control::handle_apply_external_changes(state, ch, session, request_id).await;
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
        ClientMessage::GetCommitFileDiff {
            request_id,
            commit_a,
            commit_b,
            target,
            ..
        } => {
            source_control::handle_get_commit_file_diff(
                state, ch, session, request_id, commit_a, commit_b, target,
            )
            .await;
        }
        ClientMessage::ComputeDiffProjection {
            request_id,
            revision,
            base_content,
            target_content,
            ..
        } => {
            source_control::handle_compute_diff_projection(
                state,
                ch,
                session,
                request_id,
                revision,
                base_content,
                target_content,
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
        ClientMessage::RemoteProjectionTransport {
            provider,
            direction,
            ..
        } => {
            source_control::handle_remote_projection_transport(
                state, ch, session, provider, direction,
            )
            .await;
        }
        other => super::core::route_core(state, ch, session, other).await,
    }
}

#[cfg(test)]
mod tests;
