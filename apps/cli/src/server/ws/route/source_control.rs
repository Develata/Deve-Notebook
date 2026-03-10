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
    match msg {
        ClientMessage::GetChanges => {
            source_control::handle_get_changes(state, ch, session).await;
        }
        ClientMessage::StageFile { target } => {
            source_control::handle_stage_file(state, ch, session, target).await;
        }
        ClientMessage::StageFiles { targets } => {
            source_control::handle_stage_files(state, ch, session, targets).await;
        }
        ClientMessage::UnstageFile { target } => {
            source_control::handle_unstage_file(state, ch, session, target).await;
        }
        ClientMessage::UnstageFiles { targets } => {
            source_control::handle_unstage_files(state, ch, session, targets).await;
        }
        ClientMessage::DiscardFile { target } => {
            source_control::handle_discard_file(state, ch, session, target).await;
        }
        ClientMessage::Commit { message } => {
            source_control::handle_commit(state, ch, session, message).await;
        }
        ClientMessage::GetCommitHistory { limit } => {
            source_control::handle_get_commit_history(state, ch, session, limit).await;
        }
        ClientMessage::GetCommitDiff { commit_a, commit_b } => {
            source_control::handle_get_commit_diff(state, ch, session, commit_a, commit_b).await;
        }
        ClientMessage::ResolveConflict { target, resolution } => {
            source_control::handle_resolve_conflict(state, ch, session, target, resolution).await;
        }
        ClientMessage::CommitAndPush { message } => {
            source_control::handle_commit_and_push(state, ch, session, message).await;
        }
        ClientMessage::GetDocDiff { target } => {
            source_control::handle_get_doc_diff(state, ch, session, target).await;
        }
        other => super::core::route_core(state, ch, session, other).await,
    }
}
