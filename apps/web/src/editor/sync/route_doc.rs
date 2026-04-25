use super::SyncContext;
use super::dispatch_doc;
use deve_core::protocol::ServerMessage;

pub(super) fn route_doc_message(msg: ServerMessage, ctx: &SyncContext) -> Option<ServerMessage> {
    match msg {
        ServerMessage::Snapshot {
            repo_id,
            branch,
            scope_nonce,
            doc_id: msg_doc_id,
            request_id,
            content,
            base_seq,
            version,
            delta_ops,
        } => {
            dispatch_doc::handle_snapshot_message(
                ctx,
                dispatch_doc::SnapshotDispatchMessage {
                    repo_id,
                    branch,
                    scope_nonce,
                    doc_id: msg_doc_id,
                    request_id,
                    content,
                    base_seq,
                    version,
                    delta_ops,
                },
            );
            None
        }
        ServerMessage::History {
            repo_id,
            branch,
            scope_nonce,
            doc_id: msg_doc_id,
            request_id,
            ops,
        } => {
            dispatch_doc::handle_history_message(
                ctx,
                repo_id,
                branch,
                scope_nonce,
                msg_doc_id,
                request_id,
                ops,
            );
            None
        }
        ServerMessage::NewOp {
            repo_id,
            branch,
            scope_nonce,
            doc_id: msg_doc_id,
            entry,
        } => {
            dispatch_doc::handle_new_op_message(
                ctx,
                repo_id,
                branch,
                scope_nonce,
                msg_doc_id,
                entry,
            );
            None
        }
        other => Some(other),
    }
}
