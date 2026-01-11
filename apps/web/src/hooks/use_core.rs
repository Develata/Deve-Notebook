//! # 核心状态 Hook
//!
//! 本模块提供 `use_core` Hook，管理应用的核心状态。
//!
//! ## 功能
//! - WebSocket 服务实例化
//! - 文档列表管理
//! - 文档 CRUD 操作回调
//! - 编辑器统计信息

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
    pub on_stats: Callback<EditorStats>,
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

    // Initial List Request
    let ws_clone = ws.clone();
    Effect::new(move |_| {
         ws_clone.send(ClientMessage::ListDocs);
    });
    
    // ... (rest same, just struct init changes)


    // Handle Messages
    Effect::new(move |_| {
        if let Some(msg) = ws.msg.get() {
            if let ServerMessage::DocList { docs: list } = msg {
                set_docs.set(list.clone());
                // Auto-select first if none selected
                if current_doc.get_untracked().is_none() {
                    if let Some(first) = list.first() {
                        set_current_doc.set(Some(first.0));
                    }
                }
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
    
    let on_stats = Callback::new(move |s| set_stats.set(s));

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
        on_stats,
    }
}
