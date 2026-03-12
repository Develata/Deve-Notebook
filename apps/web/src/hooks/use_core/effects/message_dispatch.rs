use crate::api::WsService;
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;

use super::super::apply::apply_tree_delta;
use super::super::effects_msg;
use super::super::effects_sc;
use super::super::pending;
use super::super::state::CoreSignals;
use super::message_protocol::handle_protocol_error;
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
            if !effects_sc::matches_current_scope(
                &repo_id,
                &branch,
                signals.current_repo_id,
                signals.active_branch,
            ) {
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
        } => signals
            .set_plugin_response
            .set(Some((req_id, result, error))),
        ServerMessage::ChatChunk {
            req_id,
            delta,
            finish_reason,
        } => effects_msg::handle_chat_chunk(
            req_id,
            delta,
            finish_reason,
            signals.set_chat_messages,
            signals.set_is_chat_streaming,
        ),
        ServerMessage::SearchResults { results } => signals.set_search_results.set(results),
        ServerMessage::SyncModeStatus { repo_id, mode } => {
            if !effects_sc::matches_current_repo(&repo_id, signals.current_repo_id) {
                return;
            }
            signals.set_sync_mode.set(mode);
        }
        ServerMessage::PendingOpsInfo {
            repo_id,
            count,
            previews,
        } => {
            if !effects_sc::matches_current_repo(&repo_id, signals.current_repo_id) {
                return;
            }
            signals.set_pending_ops_count.set(count);
            signals.set_pending_ops_previews.set(previews);
        }
        ServerMessage::MergeComplete {
            repo_id,
            merged_count,
        } => {
            if !effects_sc::matches_current_repo(&repo_id, signals.current_repo_id) {
                return;
            }
            leptos::logging::log!("已合并 {} 个操作", merged_count);
            signals.set_pending_ops_count.set(0);
            signals.set_pending_ops_previews.set(vec![]);
        }
        ServerMessage::PendingDiscarded { repo_id } => {
            if !effects_sc::matches_current_repo(&repo_id, signals.current_repo_id) {
                return;
            }
            leptos::logging::log!("待处理操作已丢弃");
            signals.set_pending_ops_count.set(0);
            signals.set_pending_ops_previews.set(vec![]);
        }
        ServerMessage::ShadowList { shadows } => signals.set_shadow_repos.set(shadows),
        ServerMessage::RepoList { branch, repos } => {
            let current_branch = signals
                .active_branch
                .get_untracked()
                .map(|id| id.to_string());
            if branch == current_branch {
                signals.set_repo_list.set(repos);
            }
        }
        ServerMessage::BranchSwitched { peer_id, success } => {
            if effects_msg::handle_branch_switched(
                peer_id,
                success,
                signals.active_branch,
                signals.pending_branch_switch,
                signals.set_pending_branch_switch,
                signals.set_active_branch,
            ) {
                ws.clear_writer_ready();
                signals.set_handshake_ready.set(false);
                signals.set_pending_repo_switch.set(None);
                signals.set_current_repo.set(None);
                signals.set_current_repo_id.set(None);
                signals.set_current_doc.set(None);
                signals.set_docs.set(Vec::new());
                signals.set_tree_nodes.set(Vec::new());
                signals.set_repo_list.set(Vec::new());
                effects_sc::clear_repo_scoped_state(
                    signals.set_staged_changes,
                    signals.set_unstaged_changes,
                    signals.set_commit_history,
                    signals.set_diff_content,
                    signals.set_commit_diff_result,
                );
            }
        }
        ServerMessage::RepoSwitched { name, uuid } => {
            ws.clear_writer_ready();
            signals.set_handshake_ready.set(false);
            if effects_msg::handle_repo_switched(
                name,
                uuid,
                crate::hooks::use_core::RepoSwitchSignals {
                    current_repo_id: signals.current_repo_id,
                    pending_repo_switch: signals.pending_repo_switch,
                    set_pending_repo_switch: signals.set_pending_repo_switch,
                    set_current_repo: signals.set_current_repo,
                    set_current_repo_id: signals.set_current_repo_id,
                    set_current_doc: signals.set_current_doc,
                },
            ) {
                signals.set_docs.set(Vec::new());
                signals.set_tree_nodes.set(Vec::new());
                effects_sc::clear_repo_scoped_state(
                    signals.set_staged_changes,
                    signals.set_unstaged_changes,
                    signals.set_commit_history,
                    signals.set_diff_content,
                    signals.set_commit_diff_result,
                );
            }
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
            if signals.active_branch.get_untracked().is_none()
                && branch == signals.active_branch.get_untracked()
                && signals.current_repo_id.get_untracked().as_deref() == Some(repo_id.as_str())
            {
                leptos::logging::log!("Writer ready for repo {} via {}", repo_id, peer_id);
                ws.mark_writer_ready(repo_id, peer_id.as_str());
            }
        }
        ServerMessage::TreeUpdate {
            repo_id,
            branch,
            delta,
        } => {
            if !effects_sc::matches_current_scope(
                &repo_id,
                &branch,
                signals.current_repo_id,
                signals.active_branch,
            ) {
                return;
            }
            signals
                .set_tree_nodes
                .update(|nodes| apply_tree_delta(nodes, delta));
        }
        ServerMessage::Ack {
            doc_id,
            client_op_id,
            ..
        } => signals.set_pending_local_edits.update(|pending_edits| {
            let _ = pending::ack_pending_edit(pending_edits, doc_id, client_op_id);
        }),
        other => handle_sc_or_remaining(other, ws, signals, schedule_refresh),
    }
}
