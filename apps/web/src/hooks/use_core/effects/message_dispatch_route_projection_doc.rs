use deve_core::protocol::ServerMessage;

use super::super::super::state::CoreSignals;
use super::super::message_dispatch_projection::{
    handle_doc_list_message, handle_tree_update_message,
};

pub fn route_projection_doc_message(
    msg: ServerMessage,
    signals: CoreSignals,
) -> Result<(), ServerMessage> {
    match msg {
        ServerMessage::DocList {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            docs,
        } => {
            handle_doc_list_message(request_id, repo_id, branch, scope_nonce, docs, signals);
            Ok(())
        }
        ServerMessage::TreeUpdate {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            delta,
        } => {
            handle_tree_update_message(request_id, repo_id, branch, scope_nonce, delta, signals);
            Ok(())
        }
        other => Err(other),
    }
}
