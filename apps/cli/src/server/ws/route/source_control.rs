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
        ClientMessage::StageFile { path } => {
            source_control::handle_stage_file(state, ch, path).await;
        }
        ClientMessage::StageFiles { paths } => {
            source_control::handle_stage_files(state, ch, session, paths).await;
        }
        ClientMessage::UnstageFile { path } => {
            source_control::handle_unstage_file(state, ch, path).await;
        }
        ClientMessage::UnstageFiles { paths } => {
            source_control::handle_unstage_files(state, ch, session, paths).await;
        }
        ClientMessage::DiscardFile { path } => {
            source_control::handle_discard_file(state, ch, session, path).await;
        }
        ClientMessage::Commit { message } => {
            source_control::handle_commit(state, ch, message).await;
        }
        ClientMessage::GetCommitHistory { limit } => {
            source_control::handle_get_commit_history(state, ch, limit).await;
        }
        ClientMessage::GetDocDiff { path } => {
            source_control::handle_get_doc_diff(state, ch, session, path).await;
        }
        other => super::core::route_core(state, ch, session, other).await,
    }
}
