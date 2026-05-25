//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#tree-projection-contract
//!
use deve_core::protocol::ServerMessage;

use super::super::super::state::CoreSignals;
use super::super::message_dispatch_sync::{
    handle_merge_complete_message, handle_pending_discarded_message,
    handle_pending_ops_info_message, handle_sync_hello_message, handle_sync_mode_status_message,
};

pub fn route_projection_sync_message(
    msg: ServerMessage,
    signals: CoreSignals,
) -> Option<ServerMessage> {
    match msg {
        ServerMessage::SyncHello {
            peer_id,
            repo_id,
            scope_nonce,
            vector,
            ..
        } => {
            handle_sync_hello_message(peer_id, repo_id, scope_nonce.get(), vector, signals);
            None
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
            None
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
            None
        }
        ServerMessage::MergeComplete {
            repo_id,
            branch,
            scope_nonce,
            merged_count,
        } => {
            handle_merge_complete_message(repo_id, branch, scope_nonce, merged_count, signals);
            None
        }
        ServerMessage::PendingDiscarded {
            repo_id,
            branch,
            scope_nonce,
        } => {
            handle_pending_discarded_message(repo_id, branch, scope_nonce, signals);
            None
        }
        other => Some(other),
    }
}
