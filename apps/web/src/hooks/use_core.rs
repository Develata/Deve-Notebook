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
use deve_core::models::DocId;
use deve_core::protocol::{ClientMessage, ServerMessage};
use crate::editor::EditorStats;

#[derive(Clone)]
pub struct CoreState {
    pub ws: WsService,
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>, 
    pub status_text: Signal<String>,
    pub stats: ReadSignal<EditorStats>,
    
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
}

pub fn use_core() -> CoreState {
    let ws = WsService::new();
    provide_context(ws.clone());
    
    let status_text = Signal::derive(move || format!("{}", ws.status.get()));
    
    // Global State
    let (docs, set_docs) = signal(Vec::<(DocId, String)>::new());
    let (current_doc, set_current_doc) = signal(None::<DocId>);
    // Stats State
    let (stats, set_stats) = signal(EditorStats::default());

    // Plugin State
    let (plugin_response, set_plugin_response) = signal(None::<(String, Option<serde_json::Value>, Option<String>)>);

    // Search State
    let (search_results, set_search_results) = signal(Vec::<(String, String, f32)>::new());

    // Initial List Request
    let ws_clone = ws.clone();
    Effect::new(move |_| {
         ws_clone.send(ClientMessage::ListDocs);
    });
    
    // Handle Messages
    Effect::new(move |_| {
        if let Some(msg) = ws.msg.get() {
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
                ServerMessage::PluginResponse { req_id, result, error } => {
                     set_plugin_response.set(Some((req_id, result, error)));
                },
                ServerMessage::SearchResults { results } => {
                     set_search_results.set(results);
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

    CoreState {
        ws,
        docs,
        current_doc,
        set_current_doc,
        status_text,
        stats,
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
    }
}
