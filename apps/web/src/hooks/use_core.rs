//! # Core State Hook (核心状态钩子)
//!
//! **架构作用**:
//! 管理前端全局核心状态，连接 WebSocket 服务与 UI 组件。
//!
//! **核心功能清单**:
//! - `CoreState`: 状态容器（Docs List, Current Doc, Connection Status, Stats）。
//! - `use_core`: 初始化 WebSocket，订阅消息，暴露 CRUD 回调（Select, Create, Rename, Delete, Copy, Move）。
//!
//! **类型**: Core MUST (核心必选)

use leptos::prelude::*;
use crate::api::WsService;
use std::collections::HashMap;
use deve_core::models::{DocId, PeerId, VersionVector};
use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::source_control::{ChangeEntry, CommitInfo};
use crate::editor::EditorStats;

#[derive(Clone, Debug, PartialEq)]
pub struct PeerSession {
    pub id: PeerId,
    pub vector: VersionVector,
    pub last_seen: u64, // timestamp
}

#[derive(Clone)]
pub struct CoreState {
    pub ws: WsService,
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>, 
    pub status_text: Signal<String>,
    pub stats: ReadSignal<EditorStats>,
    
    // P2P 状态
    pub peers: ReadSignal<HashMap<PeerId, PeerSession>>,
    
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
    pub on_stats: Callback<EditorStats>,
    
    // 插件 RPC
    pub plugin_last_response: ReadSignal<Option<(String, Option<serde_json::Value>, Option<String>)>>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,

    // 搜索
    pub search_results: ReadSignal<Vec<(String, String, f32)>>, // (doc_id, path, score)
    pub on_search: Callback<String>,
    
    // 手动合并
    pub sync_mode: ReadSignal<String>, // "auto" or "manual"
    pub pending_ops_count: ReadSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    
    // 版本控制状态 (Repo)
    pub active_repo: ReadSignal<Option<PeerId>>,
    pub set_active_repo: WriteSignal<Option<PeerId>>,
    
    // 分支切换状态
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,
    
    // 版本控制状态 (历史)
    pub doc_version: ReadSignal<u64>, // 当前最大版本
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>, // 当前回放视图版本
    pub set_playback_version: WriteSignal<u64>,
    
    // 旁观者模式 (Spectator Mode)
    pub is_spectator: Signal<bool>,

    // Source Control (New)
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<String>,
    pub on_unstage_file: Callback<String>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub diff_content: ReadSignal<Option<(String, String, String)>>,
    pub set_diff_content: WriteSignal<Option<(String, String, String)>>,
    pub on_get_doc_diff: Callback<String>,
}


pub fn use_core() -> CoreState {
    let ws = WsService::new();
    provide_context(ws.clone());
    
    let status_signal_for_text = ws.status;
    let status_text = Signal::derive(move || format!("{}", status_signal_for_text.get()));
    
    // 全局状态
    let (docs, set_docs) = signal(Vec::<(DocId, String)>::new());
    let (current_doc, set_current_doc) = signal(None::<DocId>);
    // 统计状态
    let (stats, set_stats) = signal(EditorStats::default());
    // P2P 状态
    let (peers, set_peers) = signal(HashMap::<PeerId, PeerSession>::new());

    // 插件状态
    let (plugin_response, set_plugin_response) = signal(None::<(String, Option<serde_json::Value>, Option<String>)>);

    // 搜索状态
    let (search_results, set_search_results) = signal(Vec::<(String, String, f32)>::new());

    // 手动合并状态
    let (sync_mode, set_sync_mode) = signal("auto".to_string());
    let (pending_ops_count, set_pending_ops_count) = signal(0u32);
    let (pending_ops_previews, set_pending_ops_previews) = signal(Vec::<(String, String, String)>::new());

    // 版本控制状态
    let (active_repo, set_active_repo) = signal(None::<PeerId>);
    // 分支切换状态
    let (shadow_repos, set_shadow_repos) = signal(Vec::<String>::new());
    // 历史状态
    let (doc_version, set_doc_version) = signal(0u64);
    let (playback_version, set_playback_version) = signal(0u64);
    // 旁观者模式 - 当查看 Shadow Repo 时为 true
    let is_spectator = Memo::new(move |_| active_repo.get().is_some());

    // Source Control (New)
    let (staged_changes, set_staged_changes) = signal(Vec::<ChangeEntry>::new());
    let (unstaged_changes, set_unstaged_changes) = signal(Vec::<ChangeEntry>::new());
    let (commit_history, set_commit_history) = signal(Vec::<CommitInfo>::new());
    // Diff 视图状态 (path, old, new)
    let (diff_content, set_diff_content) = signal(None::<(String, String, String)>);

    // 为当前会话生成临时的 PeerId
    let peer_id = PeerId::random();
    leptos::logging::log!("Frontend PeerId: {}", peer_id);

    // 初始握手与列表请求
    let ws_clone = ws.clone();
    let status_signal = ws.status;
    let pid = peer_id.clone();
    
    Effect::new(move |_| {
         if status_signal.get() == crate::api::ConnectionStatus::Connected {
             leptos::logging::log!("Connected! Sending SyncHello...");
             // 发送 P2P 握手
             ws_clone.send(ClientMessage::SyncHello { 
                 peer_id: pid.clone(), 
                 vector: VersionVector::new() // 初始为空
             });
             // 请求文档列表
             ws_clone.send(ClientMessage::ListDocs);
         }
    });
    
    // 处理消息
    let ws_rx = ws.clone();
    Effect::new(move |_| {
        if let Some(msg) = ws_rx.msg.get() {
            match msg {
                ServerMessage::DocList { docs: list } => {
                    set_docs.set(list.clone());
                    // 如果未选择，则自动选择第一个
                    if current_doc.get_untracked().is_none() {
                        if let Some(first) = list.first() {
                            set_current_doc.set(Some(first.0));
                        }
                    }
                },
                
                // 跟踪 Peers
                ServerMessage::SyncHello { peer_id, vector } => {
                    set_peers.update(|map| {
                        map.insert(peer_id.clone(), PeerSession {
                            id: peer_id.clone(),
                            vector,
                            last_seen: js_sys::Date::now() as u64
                        });
                    });
                },

                ServerMessage::PluginResponse { req_id, result, error } => {
                     set_plugin_response.set(Some((req_id, result, error)));
                },
                ServerMessage::SearchResults { results } => {
                     set_search_results.set(results);
                },
                
                // 手动合并消息
                ServerMessage::SyncModeStatus { mode } => {
                    set_sync_mode.set(mode);
                },
                ServerMessage::PendingOpsInfo { count, previews } => {
                    set_pending_ops_count.set(count);
                    set_pending_ops_previews.set(previews);
                },
                ServerMessage::MergeComplete { merged_count } => {
                    leptos::logging::log!("Merged {} operations", merged_count);
                    set_pending_ops_count.set(0);
                    set_pending_ops_previews.set(vec![]);
                },
                ServerMessage::PendingDiscarded => {
                    leptos::logging::log!("Pending operations discarded");
                    set_pending_ops_count.set(0);
                    set_pending_ops_previews.set(vec![]);
                },
                ServerMessage::ShadowList { shadows } => {
                    leptos::logging::log!("Received {} shadow repos", shadows.len());
                    set_shadow_repos.set(shadows);
                },
                ServerMessage::BranchSwitched { peer_id, success } => {
                    leptos::logging::log!("Branch switched to {:?}, success: {}", peer_id, success);
                    // 分支切换成功后重新加载当前文档
                    if success {
                        if let Some(doc_id) = current_doc.get_untracked() {
                            ws_rx.send(ClientMessage::OpenDoc { doc_id });
                        }
                    }
                },
                ServerMessage::EditRejected { reason } => {
                    leptos::logging::warn!("Edit rejected: {}", reason);
                    // TODO: 可以显示 Toast 通知用户
                },
                
                // Source Control Messages
                ServerMessage::ChangesList { staged, unstaged } => {
                    set_staged_changes.set(staged);
                    set_unstaged_changes.set(unstaged);
                },
                ServerMessage::CommitHistory { commits } => {
                    set_commit_history.set(commits);
                },
                ServerMessage::StageAck { path } => {
                    leptos::logging::log!("Staged: {}", path);
                    // Refresh changes
                    ws_rx.send(ClientMessage::GetChanges);
                },
                ServerMessage::UnstageAck { path } => {
                    leptos::logging::log!("Unstaged: {}", path);
                    // Refresh changes
                    ws_rx.send(ClientMessage::GetChanges);
                },
                ServerMessage::CommitAck { commit_id, .. } => {
                     leptos::logging::log!("Committed: {}", commit_id);
                     // Refresh all
                     ws_rx.send(ClientMessage::GetChanges);
                     ws_rx.send(ClientMessage::GetCommitHistory { limit: 50 });
                },
                ServerMessage::DocDiff { path, old_content, new_content } => {
                    leptos::logging::log!("Received diff for: {}", path);
                    set_diff_content.set(Some((path, old_content, new_content)));
                },

                _ => {}
            }
        }
    });

    // 操作
    let on_doc_select = Callback::new(move |id: DocId| {
        set_current_doc.set(Some(id));
    });

    let ws_for_create = ws.clone();
    let on_doc_create = Callback::new(move |name: String| {
        ws_for_create.send(ClientMessage::CreateDoc { name });
    });

    let ws_for_rename = ws.clone();
    let on_doc_rename = Callback::new(move |(old_path, new_path): (String, String)| {
        leptos::logging::log!("App: on_doc_rename sending msg: old={} new={}", old_path, new_path);
        ws_for_rename.send(ClientMessage::RenameDoc { old_path, new_path });
    });

    let ws_for_delete = ws.clone();
    let on_doc_delete = Callback::new(move |path: String| {
        leptos::logging::log!("use_core: on_doc_delete called with path={}", path);
        ws_for_delete.send(ClientMessage::DeleteDoc { path });
    });

    let ws_for_copy = ws.clone();
    let on_doc_copy = Callback::new(move |(src_path, dest_path): (String, String)| {
        leptos::logging::log!("use_core: on_doc_copy called src={} dest={}", src_path, dest_path);
        ws_for_copy.send(ClientMessage::CopyDoc { src_path, dest_path });
    });

    let ws_for_move = ws.clone();
    let on_doc_move = Callback::new(move |(src_path, dest_path): (String, String)| {
        leptos::logging::log!("use_core: on_doc_move called src={} dest={}", src_path, dest_path);
        ws_for_move.send(ClientMessage::MoveDoc { src_path, dest_path });
    });
    
    // 监听分支切换，发送 SwitchBranch 消息
    let ws_for_branch = ws.clone();
    Effect::new(move |_| {
        let peer = active_repo.get();
        let peer_id = peer.map(|p| p.to_string());
        leptos::logging::log!("Sending SwitchBranch: {:?}", peer_id);
        ws_for_branch.send(ClientMessage::SwitchBranch { peer_id });
    });
    
    let on_stats = Callback::new(move |s| set_stats.set(s));

    let ws_for_plugin = ws.clone();
    let on_plugin_call = Callback::new(move |(req_id, plugin_id, fn_name, args): (String, String, String, Vec<serde_json::Value>)| {
        ws_for_plugin.send(ClientMessage::PluginCall { req_id, plugin_id, fn_name, args });
    });

    let ws_for_search = ws.clone();
    let on_search = Callback::new(move |query: String| {
        ws_for_search.send(ClientMessage::Search { query, limit: 50 });
    });

    // 手动合并回调
    let ws_for_get_mode = ws.clone();
    let on_get_sync_mode = Callback::new(move |_: ()| {
        ws_for_get_mode.send(ClientMessage::GetSyncMode);
    });
    
    let ws_for_set_mode = ws.clone();
    let on_set_sync_mode = Callback::new(move |mode: String| {
        ws_for_set_mode.send(ClientMessage::SetSyncMode { mode });
    });
    
    let ws_for_get_pending = ws.clone();
    let on_get_pending_ops = Callback::new(move |_: ()| {
        ws_for_get_pending.send(ClientMessage::GetPendingOps);
    });
    
    let ws_for_confirm = ws.clone();
    let on_confirm_merge = Callback::new(move |_: ()| {
        ws_for_confirm.send(ClientMessage::ConfirmMerge);
    });
    
    let ws_for_discard = ws.clone();
    let on_discard_pending = Callback::new(move |_: ()| {
        ws_for_discard.send(ClientMessage::DiscardPending);
    });

    // 分支切换回调
    let ws_for_list_shadows = ws.clone();
    let on_list_shadows = Callback::new(move |_: ()| {
        ws_for_list_shadows.send(ClientMessage::ListShadows);
    });

    // Source Control callbacks
    let ws_sc_changes = ws.clone();
    let on_get_changes = Callback::new(move |_: ()| {
        ws_sc_changes.send(ClientMessage::GetChanges);
    });

    let ws_sc_stage = ws.clone();
    let on_stage_file = Callback::new(move |path: String| {
        ws_sc_stage.send(ClientMessage::StageFile { path });
    });

    let ws_sc_unstage = ws.clone();
    let on_unstage_file = Callback::new(move |path: String| {
        ws_sc_unstage.send(ClientMessage::UnstageFile { path });
    });

    let ws_sc_commit = ws.clone();
    let on_commit = Callback::new(move |message: String| {
        ws_sc_commit.send(ClientMessage::Commit { message });
    });

    let ws_sc_history = ws.clone();
    let on_get_history = Callback::new(move |limit: u32| {
        ws_sc_history.send(ClientMessage::GetCommitHistory { limit });
    });

    let ws_sc_diff = ws.clone();
    let on_get_doc_diff = Callback::new(move |path: String| {
        ws_sc_diff.send(ClientMessage::GetDocDiff { path });
    });

    let state = CoreState {
        ws,
        docs,
        current_doc,
        set_current_doc,
        status_text,
        stats,
        peers,
        on_doc_select,
        on_doc_create,
        on_doc_rename,
        on_doc_delete,
        on_doc_copy,
        on_doc_move,
        on_stats,
        plugin_last_response: plugin_response,
        on_plugin_call,
        search_results,
        on_search,
        sync_mode,
        pending_ops_count,
        pending_ops_previews,
        on_get_sync_mode,
        on_set_sync_mode,
        on_get_pending_ops,
        on_confirm_merge,
        on_discard_pending,
        active_repo,
        set_active_repo,
        shadow_repos,
        on_list_shadows,
        doc_version,
        set_doc_version,
        playback_version,
        set_playback_version,
        is_spectator: is_spectator.into(),
        staged_changes,
        unstaged_changes,
        commit_history,
        on_get_changes,
        on_stage_file,
        on_unstage_file,
        on_commit,
        on_get_history,
        diff_content,
        set_diff_content,
        on_get_doc_diff,
    };
    
    // 为子组件提供 CoreState 上下文
    provide_context(state.clone());
    
    state
}
