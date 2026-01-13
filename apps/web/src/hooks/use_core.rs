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
    
    // P2P State
    pub peers: ReadSignal<HashMap<PeerId, PeerSession>>,
    
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
    pub on_stats: Callback<EditorStats>,
    
    // Plugin RPC
    pub plugin_last_response: ReadSignal<Option<(String, Option<serde_json::Value>, Option<String>)>>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,

    // Search
    pub search_results: ReadSignal<Vec<(String, String, f32)>>, // (doc_id, path, score)
    pub on_search: Callback<String>,
    
    // Manual Merge
    pub sync_mode: ReadSignal<String>, // "auto" or "manual"
    pub pending_ops_count: ReadSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    
    // Source Control State (Repo)
    pub active_repo: ReadSignal<Option<PeerId>>,
    pub set_active_repo: WriteSignal<Option<PeerId>>,
    
    // Branch Switcher State
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,
    
    // Source Control State (History)
    pub doc_version: ReadSignal<u64>, // Current Max Version
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>, // Current View Version
    pub set_playback_version: WriteSignal<u64>,
}


pub fn use_core() -> CoreState {
    let ws = WsService::new();
    provide_context(ws.clone());
    
    let status_signal_for_text = ws.status;
    let status_text = Signal::derive(move || format!("{}", status_signal_for_text.get()));
    
    // Global State
    let (docs, set_docs) = signal(Vec::<(DocId, String)>::new());
    let (current_doc, set_current_doc) = signal(None::<DocId>);
    // Stats State
    let (stats, set_stats) = signal(EditorStats::default());
    // P2P State
    let (peers, set_peers) = signal(HashMap::<PeerId, PeerSession>::new());

    // Plugin State
    let (plugin_response, set_plugin_response) = signal(None::<(String, Option<serde_json::Value>, Option<String>)>);

    // Search State
    let (search_results, set_search_results) = signal(Vec::<(String, String, f32)>::new());

    // Manual Merge State
    let (sync_mode, set_sync_mode) = signal("auto".to_string());
    let (pending_ops_count, set_pending_ops_count) = signal(0u32);
    let (pending_ops_previews, set_pending_ops_previews) = signal(Vec::<(String, String, String)>::new());

    // Source Control State
    let (active_repo, set_active_repo) = signal(None::<PeerId>);
    // Branch Switcher State
    let (shadow_repos, set_shadow_repos) = signal(Vec::<String>::new());
    // History State
    let (doc_version, set_doc_version) = signal(0u64);
    let (playback_version, set_playback_version) = signal(0u64);

    // Generate Ephemeral PeerId for this session
    let peer_id = PeerId::random();
    leptos::logging::log!("Frontend PeerId: {}", peer_id);

    // Initial Handshake & List Request
    let ws_clone = ws.clone();
    let status_signal = ws.status;
    let pid = peer_id.clone();
    
    Effect::new(move |_| {
         if status_signal.get() == crate::api::ConnectionStatus::Connected {
             leptos::logging::log!("Connected! Sending SyncHello...");
             // Send P2P Handshake
             ws_clone.send(ClientMessage::SyncHello { 
                 peer_id: pid.clone(), 
                 vector: VersionVector::new() // Empty initially
             });
             // Request Doc List
             ws_clone.send(ClientMessage::ListDocs);
         }
    });
    
    // Handle Messages
    let ws_rx = ws.clone();
    Effect::new(move |_| {
        if let Some(msg) = ws_rx.msg.get() {
            match msg {
                ServerMessage::DocList { docs: list } => {
                    set_docs.set(list.clone());
                    // Auto-select first if none selected
                    if current_doc.get_untracked().is_none() {
                        if let Some(first) = list.first() {
                            set_current_doc.set(Some(first.0));
                        }
                    }
                },
                
                // Track Peers
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
                
                // Manual Merge Messages
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
                _ => {}
            }
        }
    });

    // Actions
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
    
    let on_stats = Callback::new(move |s| set_stats.set(s));

    let ws_for_plugin = ws.clone();
    let on_plugin_call = Callback::new(move |(req_id, plugin_id, fn_name, args): (String, String, String, Vec<serde_json::Value>)| {
        ws_for_plugin.send(ClientMessage::PluginCall { req_id, plugin_id, fn_name, args });
    });

    let ws_for_search = ws.clone();
    let on_search = Callback::new(move |query: String| {
        ws_for_search.send(ClientMessage::Search { query, limit: 50 });
    });

    // Manual Merge Callbacks
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

    // Branch Switcher Callback
    let ws_for_list_shadows = ws.clone();
    let on_list_shadows = Callback::new(move |_: ()| {
        ws_for_list_shadows.send(ClientMessage::ListShadows);
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
    };
    
    // Provide CoreState as context for child components
    provide_context(state.clone());
    
    state
}
