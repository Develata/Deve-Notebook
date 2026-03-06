// apps/web/src/hooks/use_core/effects.rs
//! # 响应式效果 (Effects)
//!
//! 定义握手逻辑和消息处理 Effect。
//! SC 相关消息已拆分到 `effects_sc.rs`。

use crate::api::{ConnectionStatus, WsService};
use crate::storage::DegradedSyncMode;
use crate::storage::identity::{
    StoredPeerIdentity, note_handshake, save_repo_vector, sign_sync_hello,
};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::{ClientMessage, ServerMessage};
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::apply::apply_tree_delta;
use super::effects_msg;
use super::effects_sc;
use super::state::CoreSignals;

/// 设置握手 Effect。
pub fn setup_handshake_effect(
    ws: &WsService,
    identity: ReadSignal<Option<StoredPeerIdentity>>,
    repo_vector: ReadSignal<VersionVector>,
    degraded: ReadSignal<Option<DegradedSyncMode>>,
) {
    let ws_clone = ws.clone();
    let status_signal = ws.status;
    let endpoint_signal = ws.endpoint;
    let last_mode = Rc::new(RefCell::new(None::<String>));

    Effect::new(move |_| {
        if status_signal.get() != ConnectionStatus::Connected {
            *last_mode.borrow_mut() = None;
            return;
        }

        let Some(mode_key) = degraded
            .get()
            .as_ref()
            .map(|_| format!("{}::degraded", endpoint_signal.get()))
            .or_else(|| {
                identity
                    .get()
                    .as_ref()
                    .map(|id| format!("{}::{}", endpoint_signal.get(), id.repo_id))
            })
        else {
            return;
        };
        if last_mode.borrow().as_deref() == Some(mode_key.as_str()) {
            return;
        }
        *last_mode.borrow_mut() = Some(mode_key);

        let ws = ws_clone.clone();
        let maybe_mode = degraded.get();
        let maybe_identity = identity.get();
        let vector = repo_vector.get();
        spawn_local(async move {
            if let Some(mode) = maybe_mode {
                leptos::logging::warn!("{}", mode.banner_text());
                ws.send(ClientMessage::ListDocs);
                ws.send(ClientMessage::ListRepos);
                return;
            }
            let Some(identity) = maybe_identity else {
                return;
            };

            leptos::logging::log!("已连接! 发送 SyncHello...");
            let sorted_map: BTreeMap<_, _> = vector.iter().collect();
            let vec_bytes = serde_json::to_vec(&sorted_map).unwrap_or_default();
            let mut msg = Vec::new();
            msg.extend_from_slice(b"deve-handshake");
            msg.extend_from_slice(identity.peer_id.as_bytes());
            msg.extend_from_slice(&vec_bytes);

            match sign_sync_hello(&identity, &msg).await {
                Ok(signature) => {
                    let peer_id = PeerId::new(&identity.peer_id);
                    let vector_json = serde_json::to_string(&vector).unwrap_or_default();
                    let _ = save_repo_vector(&identity.repo_id, &vector_json).await;
                    let _ = note_handshake(&identity.repo_id).await;
                    ws.send(ClientMessage::SyncHello {
                        peer_id,
                        pub_key: identity.public_key.clone(),
                        signature,
                        vector,
                    });
                }
                Err(err) => leptos::logging::error!("WebCrypto 握手签名失败: {}", err),
            }
            ws.send(ClientMessage::ListDocs);
            ws.send(ClientMessage::ListRepos);
        });
    });
}

/// 设置消息处理 Effect。
pub fn setup_message_effect(ws: &WsService, signals: &CoreSignals) {
    let ws_rx = ws.clone();
    let degraded_sync_mode = signals.degraded_sync_mode;
    let set_sync_banner = signals.set_sync_banner;
    let set_docs = signals.set_docs;
    let current_doc = signals.current_doc;
    let set_current_doc = signals.set_current_doc;
    let set_peers = signals.set_peers;
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
    let set_chat_messages = signals.set_chat_messages;
    let set_is_chat_streaming = signals.set_is_chat_streaming;
    let set_system_metrics = signals.set_system_metrics;
    let current_repo = signals.current_repo;
    let changes_refresh = Rc::new(RefCell::new(None::<Timeout>));

    // 当浏览器缺少 WebCrypto / IndexedDB，或身份恢复失败时会进入降级模式。
    // UI 必须把只读约束显式暴露出来，避免用户误以为当前仍可编辑或发起写入同步。
    Effect::new(move |_| {
        let banner = degraded_sync_mode.get().map(|mode| {
            format!("存储受限（{}），当前处于只读模式", mode.reason)
        });
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
                    effects_msg::handle_doc_list(list, set_docs, current_doc, set_current_doc);
                }
                ServerMessage::SyncHello {
                    peer_id, vector, ..
                } => {
                    effects_msg::handle_sync_hello(peer_id, vector.clone(), set_peers);
                    let repo_id = current_repo
                        .get_untracked()
                        .unwrap_or_else(|| "default".to_string());
                    spawn_local(async move {
                        let vector_json = serde_json::to_string(&vector).unwrap_or_default();
                        let _ = save_repo_vector(&repo_id, &vector_json).await;
                        let _ = note_handshake(&repo_id).await;
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
                ServerMessage::MergeComplete { merged_count } => {
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
                    effects_msg::handle_branch_switched(
                        &ws_rx,
                        peer_id,
                        success,
                        current_doc,
                        set_active_branch,
                    );
                }
                ServerMessage::RepoSwitched { name, uuid: _ } => {
                    effects_msg::handle_repo_switched(&ws_rx, name, current_doc, set_current_repo);
                }
                ServerMessage::EditRejected { reason } => {
                    leptos::logging::warn!("编辑被拒绝: {}", reason);
                }
                ServerMessage::TreeUpdate(delta) => {
                    set_tree_nodes.update(|nodes| apply_tree_delta(nodes, delta));
                }
                other_sc => {
                    if !effects_sc::handle_sc_message(
                        &other_sc,
                        set_staged_changes,
                        set_unstaged_changes,
                        set_commit_history,
                        set_diff_content,
                        set_commit_diff_result,
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
