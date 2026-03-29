use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
use super::message_dispatch_projection::{handle_doc_list_message, handle_tree_update_message};
use super::message_dispatch_sync::{
    handle_merge_complete_message, handle_pending_discarded_message,
    handle_pending_ops_info_message, handle_sync_hello_message, handle_sync_mode_status_message,
};

pub fn route_projection_and_sync_message(
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
        ServerMessage::SyncHello {
            peer_id,
            repo_id,
            scope_nonce,
            vector,
            ..
        } => {
            handle_sync_hello_message(peer_id, repo_id, scope_nonce, vector, signals);
            Ok(())
        }
        ServerMessage::SyncModeStatus {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            mode,
        } => {
            handle_sync_mode_status_message(
                request_id,
                repo_id,
                branch,
                scope_nonce,
                mode,
                signals,
            );
            Ok(())
        }
        ServerMessage::PendingOpsInfo {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            count,
            previews,
        } => {
            handle_pending_ops_info_message(
                request_id,
                repo_id,
                branch,
                scope_nonce,
                count,
                previews,
                signals,
            );
            Ok(())
        }
        ServerMessage::MergeComplete {
            repo_id,
            branch,
            scope_nonce,
            merged_count,
        } => {
            handle_merge_complete_message(repo_id, branch, scope_nonce, merged_count, signals);
            Ok(())
        }
        ServerMessage::PendingDiscarded {
            repo_id,
            branch,
            scope_nonce,
        } => {
            handle_pending_discarded_message(repo_id, branch, scope_nonce, signals);
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
