use crate::api::WsService;
use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
use super::message_dispatch_control::{
    handle_branch_switched_message, handle_peer_deleted_message, handle_repo_list_message,
    handle_repo_switched_message,
};
use super::message_dispatch_protocol::{
    handle_edit_rejected_message, handle_protocol_error_message,
};
use super::message_dispatch_runtime::{
    handle_chat_chunk_message, handle_plugin_response_message, handle_search_results_message,
};
use super::message_dispatch_shadow::handle_shadow_list_message;
use super::message_dispatch_write::{handle_ack_message, handle_write_ready_message};
use super::message_projection::{handle_doc_list, handle_tree_update};
use super::message_runtime_sync::{
    handle_merge_complete, handle_pending_discarded, handle_pending_ops_info,
    handle_sync_mode_status,
};
use super::message_sync::handle_sync_hello;
use super::message_sync_dispatch::handle_sc_or_remaining;

pub fn handle_message<F>(
    msg: ServerMessage,
    ws: &WsService,
    signals: CoreSignals,
    locale: crate::i18n::Locale,
    schedule_refresh: &F,
) where
    F: Fn(),
{
    match msg {
        ServerMessage::DocList {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            docs,
        } => handle_doc_list(request_id, repo_id, branch, scope_nonce, docs, signals),
        ServerMessage::SyncHello {
            peer_id,
            repo_id,
            scope_nonce,
            vector,
            ..
        } => handle_sync_hello(peer_id, repo_id.to_string(), scope_nonce, vector, signals),
        ServerMessage::PluginResponse {
            req_id,
            result,
            error,
        } => handle_plugin_response_message(req_id, result, error, signals),
        ServerMessage::ChatChunk {
            req_id,
            delta,
            finish_reason,
        } => handle_chat_chunk_message(req_id, delta, finish_reason, signals),
        ServerMessage::SearchResults {
            request_id,
            scope_nonce,
            results,
        } => handle_search_results_message(request_id, scope_nonce, results, signals),
        ServerMessage::SyncModeStatus {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            mode,
        } => handle_sync_mode_status(request_id, repo_id, branch, scope_nonce, mode, signals),
        ServerMessage::PendingOpsInfo {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            count,
            previews,
        } => handle_pending_ops_info(
            request_id,
            repo_id,
            branch,
            scope_nonce,
            count,
            previews,
            signals,
        ),
        ServerMessage::MergeComplete {
            repo_id,
            branch,
            scope_nonce,
            merged_count,
        } => handle_merge_complete(repo_id, branch, scope_nonce, merged_count, signals),
        ServerMessage::PendingDiscarded {
            repo_id,
            branch,
            scope_nonce,
        } => handle_pending_discarded(repo_id, branch, scope_nonce, signals),
        ServerMessage::ShadowList {
            request_id,
            scope_nonce,
            shadows,
        } => handle_shadow_list_message(request_id, scope_nonce, shadows, ws, signals),
        ServerMessage::RepoList {
            request_id,
            branch,
            scope_nonce,
            repos,
        } => handle_repo_list_message(request_id, branch, scope_nonce, repos, ws, signals),
        ServerMessage::BranchSwitched {
            peer_id,
            success,
            switch_nonce,
        } => handle_branch_switched_message(peer_id, success, switch_nonce, ws, signals),
        ServerMessage::RepoSwitched {
            branch,
            name,
            uuid,
            switch_nonce,
        } => handle_repo_switched_message(branch, name, uuid, switch_nonce, ws, signals),
        ServerMessage::PeerDeleted {
            peer_id,
            scope_nonce,
        } => handle_peer_deleted_message(peer_id, scope_nonce, ws, signals),
        ServerMessage::EditRejected { scope_nonce, error } => {
            handle_edit_rejected_message(scope_nonce, error, ws, locale, signals);
        }
        ServerMessage::ProtocolError {
            error,
            switch_nonce,
            scope_nonce,
        } => handle_protocol_error_message(error, switch_nonce, scope_nonce, ws, locale, signals),
        ServerMessage::WriteReady {
            peer_id,
            repo_id,
            scope_nonce,
            branch,
        } => handle_write_ready_message(peer_id, repo_id, scope_nonce, branch, ws, signals),
        ServerMessage::TreeUpdate {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            delta,
        } => handle_tree_update(request_id, repo_id, branch, scope_nonce, delta, signals),
        ServerMessage::Ack {
            repo_id,
            branch,
            scope_nonce,
            doc_id,
            client_op_id,
            ..
        } => handle_ack_message(repo_id, branch, scope_nonce, doc_id, client_op_id, signals),
        other => handle_sc_or_remaining(other, ws, signals, schedule_refresh),
    }
}
