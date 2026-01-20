// apps/web/src/hooks/use_core/effects.rs
//! # 响应式效果 (Effects)
//!
//! 定义握手逻辑和消息处理 Effect。

use crate::api::{ConnectionStatus, WsService};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::{ClientMessage, ServerMessage};
use leptos::prelude::*;
use std::sync::Arc;

use super::apply::apply_tree_delta;
use super::state::CoreSignals;
use super::types::PeerSession;

/// 设置握手 Effect
///
/// 在连接成功后发送 P2P 握手消息及初始请求。
pub fn setup_handshake_effect(
    ws: &WsService,
    key_pair: Arc<deve_core::security::IdentityKeyPair>,
    peer_id: PeerId,
) {
    let ws_clone = ws.clone();
    let status_signal = ws.status;
    let pid = peer_id.clone();
    let kp_clone = key_pair.clone();

    Effect::new(move |_| {
        if status_signal.get() == ConnectionStatus::Connected {
            leptos::logging::log!("已连接! 发送 SyncHello...");

            // 确定性序列化: 转换为 BTreeMap (排序键)
            let local_vector = VersionVector::new();
            let sorted_map: std::collections::BTreeMap<_, _> = local_vector.iter().collect();
            let vec_bytes = serde_json::to_vec(&sorted_map).unwrap_or_default();

            let mut msg = Vec::new();
            msg.extend_from_slice(b"deve-handshake");
            msg.extend_from_slice(pid.as_str().as_bytes());
            msg.extend_from_slice(&vec_bytes);

            let signature = kp_clone.sign(&msg);

            // 发送 P2P 握手
            ws_clone.send(ClientMessage::SyncHello {
                peer_id: pid.clone(),
                pub_key: kp_clone.public_key_bytes().to_vec(),
                signature,
                vector: local_vector,
            });
            // 请求文档列表
            ws_clone.send(ClientMessage::ListDocs);
            // 请求仓库列表
            ws_clone.send(ClientMessage::ListRepos);
        }
    });
}

/// 设置消息处理 Effect
///
/// 订阅 WebSocket 消息并更新对应信号。
pub fn setup_message_effect(ws: &WsService, signals: &CoreSignals) {
    let ws_rx = ws.clone();
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
    let set_tree_nodes = signals.set_tree_nodes;

    Effect::new(move |_| {
        if let Some(msg) = ws_rx.msg.get() {
            match msg {
                ServerMessage::DocList { docs: list } => {
                    set_docs.set(list.clone());
                    if current_doc.get_untracked().is_none() {
                        if let Some(first) = list.first() {
                            set_current_doc.set(Some(first.0));
                        }
                    }
                }
                ServerMessage::SyncHello {
                    peer_id, vector, ..
                } => {
                    set_peers.update(|map| {
                        map.insert(
                            peer_id.clone(),
                            PeerSession {
                                id: peer_id.clone(),
                                vector,
                                last_seen: js_sys::Date::now() as u64,
                            },
                        );
                    });
                }
                ServerMessage::PluginResponse {
                    req_id,
                    result,
                    error,
                } => {
                    set_plugin_response.set(Some((req_id, result, error)));
                }
                ServerMessage::SearchResults { results } => {
                    set_search_results.set(results);
                }
                ServerMessage::SyncModeStatus { mode } => {
                    set_sync_mode.set(mode);
                }
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
                ServerMessage::ShadowList { shadows } => {
                    leptos::logging::log!("收到 {} 个影子库", shadows.len());
                    set_shadow_repos.set(shadows);
                }
                ServerMessage::RepoList { repos } => {
                    leptos::logging::log!("收到 {} 个仓库", repos.len());
                    set_repo_list.set(repos);
                }
                ServerMessage::BranchSwitched { peer_id, success } => {
                    leptos::logging::log!("分支已切换到 {:?}, 成功: {}", peer_id, success);
                    if success {
                        if let Some(doc_id) = current_doc.get_untracked() {
                            ws_rx.send(ClientMessage::OpenDoc { doc_id });
                        }
                    }
                }
                ServerMessage::EditRejected { reason } => {
                    leptos::logging::warn!("编辑被拒绝: {}", reason);
                }
                ServerMessage::ChangesList { staged, unstaged } => {
                    set_staged_changes.set(staged);
                    set_unstaged_changes.set(unstaged);
                }
                ServerMessage::CommitHistory { commits } => {
                    set_commit_history.set(commits);
                }
                ServerMessage::StageAck { path } => {
                    leptos::logging::log!("已暂存: {}", path);
                    ws_rx.send(ClientMessage::GetChanges);
                }
                ServerMessage::UnstageAck { path } => {
                    leptos::logging::log!("已取消暂存: {}", path);
                    ws_rx.send(ClientMessage::GetChanges);
                }
                ServerMessage::CommitAck { commit_id, .. } => {
                    leptos::logging::log!("已提交: {}", commit_id);
                    ws_rx.send(ClientMessage::GetChanges);
                    ws_rx.send(ClientMessage::GetCommitHistory { limit: 50 });
                }
                ServerMessage::DocDiff {
                    path,
                    old_content,
                    new_content,
                } => {
                    leptos::logging::log!("收到 Diff: {}", path);
                    set_diff_content.set(Some((path, old_content, new_content)));
                }
                ServerMessage::TreeUpdate(delta) => {
                    leptos::logging::log!("收到 TreeUpdate: {:?}", delta);
                    set_tree_nodes.update(|nodes| {
                        apply_tree_delta(nodes, delta);
                    });
                }
                _ => {}
            }
        }
    });
}

/// 设置分支切换 Effect
pub fn setup_branch_switch_effect(ws: &WsService, active_repo: ReadSignal<Option<PeerId>>) {
    let ws_for_branch = ws.clone();
    Effect::new(move |_| {
        let peer = active_repo.get();
        let peer_id = peer.map(|p| p.to_string());
        leptos::logging::log!("发送 SwitchBranch: {:?}", peer_id);
        ws_for_branch.send(ClientMessage::SwitchBranch { peer_id });
    });
}
