use crate::server::handlers::docs;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

/// 路由文档结构相关消息（创建/重命名/删除/复制/移动）。
pub(super) async fn route_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::CreateDoc { name } => {
            docs::handle_create_doc(state, ch, session, name).await;
        }
        ClientMessage::RenameDoc { old_path, new_path } => {
            docs::handle_rename_doc(state, ch, session, old_path, new_path).await;
        }
        ClientMessage::DeleteDoc { path } => {
            docs::handle_delete_doc(state, ch, session, path).await;
        }
        ClientMessage::CopyDoc {
            src_path,
            dest_path,
        } => {
            docs::handle_copy_doc(state, ch, session, src_path, dest_path).await;
        }
        ClientMessage::MoveDoc {
            src_path,
            dest_path,
        } => {
            docs::handle_move_doc(state, ch, session, src_path, dest_path).await;
        }
        other => super::merge::route_merge(state, ch, session, other).await,
    }
}

