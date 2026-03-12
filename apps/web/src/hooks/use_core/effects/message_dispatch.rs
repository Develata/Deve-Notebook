use crate::api::WsService;
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;

use super::super::apply::apply_tree_delta;
use super::super::effects_msg;
use super::super::pending;
use super::super::state::CoreSignals;
use super::message_control;
use super::message_dispatch_gate::{
    accepts_chat_chunk, accepts_plugin_response, accepts_search_results,
};
use super::message_protocol::handle_protocol_error;
use super::message_repo_scope::{accepts_write_ready_message, matches_current_message_scope};
use super::message_scope::{repo_list_matches_scope, shadow_list_matches_scope};
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
            repo_id,
            branch,
            docs,
        } => {
            if !matches_current_message_scope(&repo_id, &branch, signals) {
                return;
            }
            effects_msg::handle_doc_list(docs, signals.set_docs);
        }
        ServerMessage::SyncHello {
            peer_id,
            repo_id,
            vector,
            ..
        } => handle_sync_hello(peer_id, repo_id.to_string(), vector, signals),
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
            repo_id,
            branch,
            mode,
        } => {
            if !matches_current_message_scope(&repo_id, &branch, signals) {
                return;
            }
            signals.set_sync_mode.set(mode);
        }
        ServerMessage::PendingOpsInfo {
            repo_id,
            branch,
            count,
            previews,
        } => {
            if !matches_current_message_scope(&repo_id, &branch, signals) {
                return;
            }
            signals.set_pending_ops_count.set(count);
            signals.set_pending_ops_previews.set(previews);
        }
        ServerMessage::MergeComplete {
            repo_id,
            branch,
            merged_count,
        } => {
            if !matches_current_message_scope(&repo_id, &branch, signals) {
                return;
            }
            leptos::logging::log!("已合并 {} 个操作", merged_count);
            signals.set_pending_ops_count.set(0);
            signals.set_pending_ops_previews.set(vec![]);
        }
        ServerMessage::PendingDiscarded { repo_id, branch } => {
            if !matches_current_message_scope(&repo_id, &branch, signals) {
                return;
            }
            leptos::logging::log!("待处理操作已丢弃");
            signals.set_pending_ops_count.set(0);
            signals.set_pending_ops_previews.set(vec![]);
        }
        ServerMessage::ShadowList {
            request_id,
            shadows,
        } => {
            if shadow_list_matches_scope(
                request_id.clone(),
                signals.pending_branch_switch.get_untracked(),
                signals.pending_repo_switch.get_untracked(),
                signals.shadow_list_request_id.get_untracked(),
            ) {
                if request_id.is_some() {
                    signals.set_shadow_list_request_id.set(None);
                }
                signals.set_shadow_repos.set(shadows);
            }
        }
        ServerMessage::RepoList {
            request_id,
            branch,
            repos,
        } => {
            if repo_list_matches_scope(
                request_id.clone(),
                branch,
                signals.active_branch.get_untracked(),
                signals.pending_branch_switch.get_untracked(),
                signals.pending_repo_switch.get_untracked(),
                signals.repo_list_request_id.get_untracked(),
            ) {
                if request_id.is_some() {
                    signals.set_repo_list_request_id.set(None);
                }
                signals.set_repo_list.set(repos);
            }
        }
        ServerMessage::BranchSwitched { peer_id, success } => {
            message_control::handle_branch_switched(peer_id, success, ws, signals);
        }
        ServerMessage::RepoSwitched { branch, name, uuid } => {
            message_control::handle_repo_switched(branch, name, uuid, ws, signals);
        }
        ServerMessage::PeerDeleted { peer_id } => {
            message_control::handle_peer_deleted(peer_id, ws, signals);
        }
        ServerMessage::EditRejected { error } | ServerMessage::ProtocolError { error } => {
            handle_protocol_error(ws, locale, &error);
        }
        ServerMessage::WriteReady {
            peer_id,
            repo_id,
            branch,
        } => {
            let repo_id = repo_id.to_string();
            if accepts_write_ready_message(&repo_id, &branch, signals) {
                leptos::logging::log!("Writer ready for repo {} via {}", repo_id, peer_id);
                ws.mark_writer_ready(repo_id, peer_id.as_str());
            }
        }
        ServerMessage::TreeUpdate {
            repo_id,
            branch,
            delta,
        } => {
            if !matches_current_message_scope(&repo_id, &branch, signals) {
                return;
            }
            signals
                .set_tree_nodes
                .update(|nodes| apply_tree_delta(nodes, delta));
        }
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
