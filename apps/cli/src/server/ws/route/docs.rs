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
    if let Some(scope_nonce) = requested_scope_nonce(&msg)
        && let Err(error) =
            super::scope_guard::validate_browser_scope_nonce(session, scope_nonce, "document")
    {
        ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
        return;
    }
    match msg {
        ClientMessage::CreateDoc { name, .. } => {
            docs::handle_create_doc(state, ch, session, name).await;
        }
        ClientMessage::RenameDoc {
            old_path, new_path, ..
        } => {
            docs::handle_rename_doc(state, ch, session, old_path, new_path).await;
        }
        ClientMessage::DeleteDoc { path, .. } => {
            docs::handle_delete_doc(state, ch, session, path).await;
        }
        ClientMessage::CopyDoc {
            src_path,
            dest_path,
            ..
        } => {
            docs::handle_copy_doc(state, ch, session, src_path, dest_path).await;
        }
        ClientMessage::MoveDoc {
            src_path,
            dest_path,
            ..
        } => {
            docs::handle_move_doc(state, ch, session, src_path, dest_path).await;
        }
        other => super::merge::route_merge(state, ch, session, other).await,
    }
}

fn requested_scope_nonce(msg: &ClientMessage) -> Option<Option<u64>> {
    match msg {
        ClientMessage::CreateDoc { scope_nonce, .. }
        | ClientMessage::RenameDoc { scope_nonce, .. }
        | ClientMessage::DeleteDoc { scope_nonce, .. }
        | ClientMessage::CopyDoc { scope_nonce, .. }
        | ClientMessage::MoveDoc { scope_nonce, .. } => Some(*scope_nonce),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::requested_scope_nonce;
    use deve_core::protocol::ClientMessage;

    #[test]
    fn extracts_scope_nonce_from_doc_messages() {
        assert_eq!(
            requested_scope_nonce(&ClientMessage::CreateDoc {
                name: "notes".into(),
                scope_nonce: Some(3),
            }),
            Some(Some(3))
        );
        assert_eq!(
            requested_scope_nonce(&ClientMessage::MoveDoc {
                src_path: "a.md".into(),
                dest_path: "b.md".into(),
                scope_nonce: Some(5),
            }),
            Some(Some(5))
        );
        assert_eq!(requested_scope_nonce(&ClientMessage::Ping), None);
    }
}
