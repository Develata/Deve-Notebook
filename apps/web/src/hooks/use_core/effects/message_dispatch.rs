use crate::api::WsService;
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;

use super::super::effects_msg;
use super::super::pending;
use super::super::state::CoreSignals;
use super::message_control;
use super::message_dispatch_gate::{
    accepts_chat_chunk, accepts_plugin_response, accepts_search_results,
};
use super::message_projection::{handle_doc_list, handle_tree_update};
use super::message_protocol::{ProtocolControlSignals, handle_protocol_error};
use super::message_repo_scope::{accepts_write_ready_message, matches_current_message_scope};
use super::message_runtime::{
    handle_merge_complete, handle_pending_discarded, handle_pending_ops_info,
    handle_sync_mode_status,
};
use super::message_scope::{
    RepoListScope, RequestMatch, ShadowListScope, repo_list_matches_scope,
    shadow_list_matches_scope,
};
use super::message_shadow;
use super::message_sync::{handle_sc_or_remaining, handle_sync_hello};

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
        } => {
            if !accepts_plugin_response(&req_id, signals) {
                return;
            }
            signals
                .set_plugin_request_ids
                .update(|ids| ids.retain(|id| id != &req_id));
            signals
                .set_plugin_response
                .set(Some((req_id, result, error)));
        }
        ServerMessage::ChatChunk {
            req_id,
            delta,
            finish_reason,
        } => {
            if !accepts_chat_chunk(&req_id, signals) {
                return;
            }
            effects_msg::handle_chat_chunk(
                req_id,
                delta,
                finish_reason,
                signals.set_chat_messages,
                signals.set_is_chat_streaming,
            );
        }
        ServerMessage::SearchResults {
            request_id,
            results,
        } => {
            if !accepts_search_results(&request_id, signals) {
                return;
            }
            signals.set_search_results.set(results);
        }
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
        } => {
            if shadow_list_matches_scope(
                RequestMatch {
                    message_id: request_id.as_deref(),
                    expected_id: signals.shadow_list_request_id.get_untracked().as_deref(),
                    scope_nonce,
                    current_scope_nonce: signals.current_scope_nonce.get_untracked(),
                },
                &ShadowListScope {
                    pending_branch_switch: signals.pending_branch_switch.get_untracked(),
                    pending_repo_switch: signals.pending_repo_switch.get_untracked(),
                },
            ) {
                if request_id.is_some() {
                    signals.set_shadow_list_request_id.set(None);
                }
                message_shadow::handle_shadow_list(shadows, request_id.is_some(), ws, signals);
            }
        }
        ServerMessage::RepoList {
            request_id,
            branch,
            scope_nonce,
            repos,
        } => {
            if repo_list_matches_scope(
                RequestMatch {
                    message_id: request_id.as_deref(),
                    expected_id: signals.repo_list_request_id.get_untracked().as_deref(),
                    scope_nonce,
                    current_scope_nonce: signals.current_scope_nonce.get_untracked(),
                },
                branch,
                &RepoListScope {
                    active_branch: signals.active_branch.get_untracked(),
                    pending_branch_switch: signals.pending_branch_switch.get_untracked(),
                    pending_repo_switch: signals.pending_repo_switch.get_untracked(),
                },
            ) {
                if request_id.is_some() {
                    signals.set_repo_list_request_id.set(None);
                }
                signals.set_repo_list.set(repos);
            }
        }
        ServerMessage::BranchSwitched {
            peer_id,
            success,
            switch_nonce,
        } => {
            message_control::handle_branch_switched(peer_id, success, switch_nonce, ws, signals);
        }
        ServerMessage::RepoSwitched {
            branch,
            name,
            uuid,
            switch_nonce,
        } => {
            message_control::handle_repo_switched(branch, name, uuid, switch_nonce, ws, signals);
        }
        ServerMessage::PeerDeleted { peer_id } => {
            message_shadow::handle_peer_deleted(peer_id, ws, signals);
        }
        ServerMessage::EditRejected { error } => {
            handle_protocol_error(
                ws,
                locale,
                &error,
                None,
                ProtocolControlSignals {
                    pending_branch_switch: signals.pending_branch_switch,
                    pending_branch_switch_nonce: signals.pending_branch_switch_nonce,
                    set_pending_branch_switch: signals.set_pending_branch_switch,
                    set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
                    pending_repo_switch_nonce: signals.pending_repo_switch_nonce,
                    set_pending_repo_switch: signals.set_pending_repo_switch,
                    set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
                    set_shadow_list_request_id: signals.set_shadow_list_request_id,
                    set_repo_list_request_id: signals.set_repo_list_request_id,
                    set_doc_list_request_id: signals.set_doc_list_request_id,
                    set_tree_request_id: signals.set_tree_request_id,
                },
            );
        }
        ServerMessage::ProtocolError {
            error,
            switch_nonce,
        } => {
            handle_protocol_error(
                ws,
                locale,
                &error,
                switch_nonce,
                ProtocolControlSignals {
                    pending_branch_switch: signals.pending_branch_switch,
                    pending_branch_switch_nonce: signals.pending_branch_switch_nonce,
                    set_pending_branch_switch: signals.set_pending_branch_switch,
                    set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
                    pending_repo_switch_nonce: signals.pending_repo_switch_nonce,
                    set_pending_repo_switch: signals.set_pending_repo_switch,
                    set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
                    set_shadow_list_request_id: signals.set_shadow_list_request_id,
                    set_repo_list_request_id: signals.set_repo_list_request_id,
                    set_doc_list_request_id: signals.set_doc_list_request_id,
                    set_tree_request_id: signals.set_tree_request_id,
                },
            );
        }
        ServerMessage::WriteReady {
            peer_id,
            repo_id,
            scope_nonce,
            branch,
        } => {
            let repo_id = repo_id.to_string();
            if accepts_write_ready_message(&repo_id, &branch, scope_nonce, signals) {
                leptos::logging::log!("Writer ready for repo {} via {}", repo_id, peer_id);
                ws.mark_writer_ready(repo_id, peer_id.as_str());
            }
        }
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
            doc_id,
            client_op_id,
            ..
        } => {
            if !matches_current_message_scope(&Some(repo_id), &branch, signals) {
                return;
            }
            signals.set_pending_local_edits.update(|pending_edits| {
                let _ = pending::ack_pending_edit(pending_edits, doc_id, client_op_id);
            });
        }
        other => handle_sc_or_remaining(other, ws, signals, schedule_refresh),
    }
}
