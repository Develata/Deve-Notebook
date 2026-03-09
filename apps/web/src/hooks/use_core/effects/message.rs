use crate::api::WsService;
use crate::storage::identity::{note_handshake, save_repo_vector};
use deve_core::protocol::{ClientMessage, ServerMessage};
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::rc::Rc;

use super::super::apply::apply_tree_delta;
use super::super::effects_msg;
use super::super::effects_sc;
use super::super::pending;
use super::super::state::CoreSignals;

/// 设置消息处理 Effect。
pub fn setup(ws: &WsService, signals: &CoreSignals) {
    let ws_rx = ws.clone();
    let degraded_sync_mode = signals.degraded_sync_mode;
    let set_sync_banner = signals.set_sync_banner;
    let set_docs = signals.set_docs;
    let set_current_doc = signals.set_current_doc;
    let set_peers = signals.set_peers;
    let set_handshake_ready = signals.set_handshake_ready;
    let set_pending_local_edits = signals.set_pending_local_edits;
    let set_plugin_response = signals.set_plugin_response;
    let set_search_results = signals.set_search_results;
    let set_sync_mode = signals.set_sync_mode;
    let set_pending_ops_count = signals.set_pending_ops_count;
    let set_pending_ops_previews = signals.set_pending_ops_previews;
    let set_shadow_repos = signals.set_shadow_repos;
    let set_repo_list = signals.set_repo_list;
    let set_staged_changes = signals.set_staged_changes;
    let set_unstaged_changes = signals.set_unstaged_changes;
    let set_commit_history = signals.set_commit_history;
    let set_diff_content = signals.set_diff_content;
    let set_commit_diff_result = signals.set_commit_diff_result;
    let set_tree_nodes = signals.set_tree_nodes;
    let set_active_branch = signals.set_active_branch;
    let set_current_repo = signals.set_current_repo;
    let set_current_repo_id = signals.set_current_repo_id;
    let set_chat_messages = signals.set_chat_messages;
    let set_is_chat_streaming = signals.set_is_chat_streaming;
    let set_system_metrics = signals.set_system_metrics;
    let current_repo_id = signals.current_repo_id;
    let changes_refresh = Rc::new(RefCell::new(None::<Timeout>));

    Effect::new(move |_| {
        let banner = degraded_sync_mode
            .get()
            .map(|mode| format!("存储受限（{}），当前处于只读模式", mode.reason));
        set_sync_banner.set(banner);
    });

    Effect::new(move |_| {
        let schedule_refresh = {
            let changes_refresh = changes_refresh.clone();
            let ws = ws_rx.clone();
            move || {
                if let Some(t) = changes_refresh.borrow_mut().take() {
                    t.cancel();
                }
                let ws_for_timer = ws.clone();
                let timer = Timeout::new(120, move || {
                    ws_for_timer.send(ClientMessage::GetChanges);
                });
                *changes_refresh.borrow_mut() = Some(timer);
            }
        };

        if let Some(msg) = ws_rx.msg.get() {
            match msg {
                ServerMessage::DocList { docs: list } => {
                    effects_msg::handle_doc_list(list, set_docs);
                }
                ServerMessage::SyncHello {
                    peer_id,
                    repo_id,
                    vector,
                    ..
                } => {
                    effects_msg::handle_sync_hello(peer_id, vector.clone(), set_peers);
                    let repo_id_str = repo_id.to_string();
                    let matches_current = match current_repo_id.get_untracked() {
                        Some(current) => current == repo_id_str,
                        None => true,
                    };
                    if matches_current {
                        set_handshake_ready.set(true);
                    }
                    spawn_local(async move {
                        let vector_json = serde_json::to_string(&vector).unwrap_or_default();
                        let _ = save_repo_vector(&repo_id_str, &vector_json).await;
                        let _ = note_handshake(&repo_id_str).await;
                    });
                }
                ServerMessage::PluginResponse {
                    req_id,
                    result,
                    error,
                } => set_plugin_response.set(Some((req_id, result, error))),
                ServerMessage::ChatChunk {
                    req_id,
                    delta,
                    finish_reason,
                } => {
                    effects_msg::handle_chat_chunk(
                        req_id,
                        delta,
                        finish_reason,
                        set_chat_messages,
                        set_is_chat_streaming,
                    );
                }
                ServerMessage::SearchResults { results } => set_search_results.set(results),
                ServerMessage::SyncModeStatus { mode } => set_sync_mode.set(mode),
                ServerMessage::PendingOpsInfo { count, previews } => {
                    set_pending_ops_count.set(count);
                    set_pending_ops_previews.set(previews);
                }
                ServerMessage::MergeComplete {
                    repo_id,
                    merged_count,
                } => {
                    if !effects_sc::matches_current_repo(&repo_id, current_repo_id) {
                        return;
                    }
                    leptos::logging::log!("已合并 {} 个操作", merged_count);
                    set_pending_ops_count.set(0);
                    set_pending_ops_previews.set(vec![]);
                }
                ServerMessage::PendingDiscarded => {
                    leptos::logging::log!("待处理操作已丢弃");
                    set_pending_ops_count.set(0);
                    set_pending_ops_previews.set(vec![]);
                }
                ServerMessage::ShadowList { shadows } => set_shadow_repos.set(shadows),
                ServerMessage::RepoList { repos } => set_repo_list.set(repos),
                ServerMessage::BranchSwitched { peer_id, success } => {
                    effects_msg::handle_branch_switched(peer_id, success, set_active_branch);
                }
                ServerMessage::RepoSwitched { name, uuid } => {
                    set_handshake_ready.set(false);
                    effects_msg::handle_repo_switched(
                        name,
                        uuid,
                        set_current_repo,
                        set_current_repo_id,
                        set_current_doc,
                    );
                }
                ServerMessage::EditRejected { reason } => {
                    leptos::logging::warn!("编辑被拒绝: {}", reason);
                }
                ServerMessage::TreeUpdate(delta) => {
                    set_tree_nodes.update(|nodes| apply_tree_delta(nodes, delta));
                }
                ServerMessage::Ack {
                    doc_id,
                    client_op_id,
                    ..
                } => {
                    set_pending_local_edits.update(|pending_edits| {
                        let _ = pending::ack_pending_edit(pending_edits, doc_id, client_op_id);
                    });
                }
                ServerMessage::Error(error) => {
                    leptos::logging::error!("Server error: {}", error);
                }
                other_sc => {
                    if !effects_sc::handle_sc_message(
                        &other_sc,
                        set_staged_changes,
                        set_unstaged_changes,
                        set_commit_history,
                        set_diff_content,
                        set_commit_diff_result,
                        current_repo_id,
                        &schedule_refresh,
                        &ws_rx,
                    ) {
                        effects_msg::handle_remaining(other_sc, set_system_metrics);
                    }
                }
            }
        }
    });
}
