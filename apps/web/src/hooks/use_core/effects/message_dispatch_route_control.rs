use crate::api::WsService;
use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
use super::message_dispatch_control::{
    handle_branch_switched_message, handle_peer_deleted_message, handle_repo_list_message,
    handle_repo_switched_message,
};
use super::message_dispatch_runtime::{
    handle_chat_chunk_message, handle_plugin_response_message, handle_search_results_message,
};
use super::message_dispatch_shadow::handle_shadow_list_message;

pub fn route_control_and_runtime_message(
    msg: ServerMessage,
    ws: &WsService,
    signals: CoreSignals,
) -> Result<(), ServerMessage> {
    match msg {
        ServerMessage::PluginResponse {
            req_id,
            result,
            error,
        } => {
            handle_plugin_response_message(req_id, result, error, signals);
            Ok(())
        }
        ServerMessage::ChatChunk {
            req_id,
            delta,
            finish_reason,
        } => {
            handle_chat_chunk_message(req_id, delta, finish_reason, signals);
            Ok(())
        }
        ServerMessage::SearchResults {
            request_id,
            scope_nonce,
            results,
        } => {
            handle_search_results_message(request_id, scope_nonce, results, signals);
            Ok(())
        }
        ServerMessage::ShadowList {
            request_id,
            scope_nonce,
            shadows,
        } => {
            handle_shadow_list_message(request_id, scope_nonce, shadows, ws, signals);
            Ok(())
        }
        ServerMessage::RepoList {
            request_id,
            branch,
            scope_nonce,
            repos,
        } => {
            handle_repo_list_message(request_id, branch, scope_nonce, repos, ws, signals);
            Ok(())
        }
        ServerMessage::BranchSwitched {
            peer_id,
            success,
            switch_nonce,
        } => {
            handle_branch_switched_message(peer_id, success, switch_nonce, ws, signals);
            Ok(())
        }
        ServerMessage::RepoSwitched {
            branch,
            name,
            uuid,
            switch_nonce,
        } => {
            handle_repo_switched_message(branch, name, uuid, switch_nonce, ws, signals);
            Ok(())
        }
        ServerMessage::PeerDeleted {
            peer_id,
            scope_nonce,
        } => {
            handle_peer_deleted_message(peer_id, scope_nonce, ws, signals);
            Ok(())
        }
        other => Err(other),
    }
}
