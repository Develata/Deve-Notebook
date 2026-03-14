// apps/web/src/hooks/use_core/callbacks.rs
//! # 回调函数定义
//!
//! 定义所有用户交互回调 (文档 CRUD, 插件, 搜索, 同步, 版本控制)。
//!
//! Source Control 相关回调已迁移到 `callbacks_sc.rs`。

use crate::api::WsService;
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::callbacks_scope::{LocalScopeSignals, run_if_stable_local_scope};
pub use super::callbacks_switch::{SwitchCallbacks, create_switch_callbacks};
pub use super::callbacks_sync::{SyncCallbacks, create_sync_callbacks};
use super::navigation::{NavigationTarget, PendingNavigation, guard_navigation};
use super::pending::PendingLocalEdits;

// Re-export from submodule
#[allow(unused_imports)] // SourceControlCallbacks 为外部模块预留
pub use super::callbacks_sc::{SourceControlCallbacks, create_source_control_callbacks};

/// 文档操作回调
pub struct DocCallbacks {
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
}

/// 创建文档操作回调
pub fn create_doc_callbacks(
    ws: &WsService,
    current_doc: ReadSignal<Option<DocId>>,
    local_scope: LocalScopeSignals,
    pending_local_edits: ReadSignal<PendingLocalEdits>,
    set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    set_current_doc: WriteSignal<Option<DocId>>,
    set_explicit_home: WriteSignal<bool>,
) -> DocCallbacks {
    let on_doc_select = Callback::new(move |id: DocId| {
        if current_doc.get_untracked() == Some(id) {
            set_explicit_home.set(false);
            set_current_doc.set(Some(id));
            return;
        }
        let action = Callback::new(move |_: ()| {
            set_explicit_home.set(false);
            set_current_doc.set(Some(id));
        });
        let _ = guard_navigation(
            current_doc.get_untracked(),
            &pending_local_edits.get_untracked(),
            set_pending_navigation,
            NavigationTarget::Doc,
            action,
        );
    });

    let ws_for_create = ws.clone();
    let on_doc_create = Callback::new(move |name: String| {
        let ws = ws_for_create.clone();
        run_if_stable_local_scope(local_scope, "CreateDoc", move || {
            ws.send(ClientMessage::CreateDoc { name });
        });
    });

    let ws_for_rename = ws.clone();
    let on_doc_rename = Callback::new(move |(old_path, new_path): (String, String)| {
        let ws = ws_for_rename.clone();
        run_if_stable_local_scope(local_scope, "RenameDoc", move || {
            leptos::logging::log!("重命名: {} -> {}", old_path, new_path);
            ws.send(ClientMessage::RenameDoc { old_path, new_path });
        });
    });

    let ws_for_delete = ws.clone();
    let on_doc_delete = Callback::new(move |path: String| {
        let ws = ws_for_delete.clone();
        run_if_stable_local_scope(local_scope, "DeleteDoc", move || {
            leptos::logging::log!("删除: {}", path);
            ws.send(ClientMessage::DeleteDoc { path });
        });
    });

    let ws_for_copy = ws.clone();
    let on_doc_copy = Callback::new(move |(src_path, dest_path): (String, String)| {
        let ws = ws_for_copy.clone();
        run_if_stable_local_scope(local_scope, "CopyDoc", move || {
            leptos::logging::log!("复制: {} -> {}", src_path, dest_path);
            ws.send(ClientMessage::CopyDoc {
                src_path,
                dest_path,
            });
        });
    });

    let ws_for_move = ws.clone();
    let on_doc_move = Callback::new(move |(src_path, dest_path): (String, String)| {
        let ws = ws_for_move.clone();
        run_if_stable_local_scope(local_scope, "MoveDoc", move || {
            leptos::logging::log!("移动: {} -> {}", src_path, dest_path);
            ws.send(ClientMessage::MoveDoc {
                src_path,
                dest_path,
            });
        });
    });

    DocCallbacks {
        on_doc_select,
        on_doc_create,
        on_doc_rename,
        on_doc_delete,
        on_doc_copy,
        on_doc_move,
    }
}

/// 其他回调 (插件, 搜索, 统计)
pub struct MiscCallbacks {
    pub on_stats: Callback<crate::editor::EditorStats>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,
    pub on_search: Callback<String>,
}

/// 创建其他回调
pub fn create_misc_callbacks(
    ws: &WsService,
    set_stats: WriteSignal<crate::editor::EditorStats>,
    load_state: ReadSignal<String>,
    set_plugin_request_ids: WriteSignal<Vec<String>>,
    set_search_request_id: WriteSignal<Option<String>>,
) -> MiscCallbacks {
    let on_stats = Callback::new(move |s| set_stats.set(s));

    let ws_plugin = ws.clone();
    let on_plugin_call = Callback::new(
        move |(req_id, plugin_id, fn_name, args): (
            String,
            String,
            String,
            Vec<serde_json::Value>,
        )| {
            set_plugin_request_ids.update(|ids| {
                if !ids.iter().any(|id| id == &req_id) {
                    ids.push(req_id.clone());
                }
            });
            ws_plugin.send(ClientMessage::PluginCall {
                req_id,
                plugin_id,
                fn_name,
                args,
            });
        },
    );

    let ws_search = ws.clone();
    let on_search = Callback::new(move |query: String| {
        if load_state.get_untracked() != "ready" {
            leptos::logging::warn!("Search disabled while loading");
            return;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        set_search_request_id.set(Some(request_id.clone()));
        ws_search.send(ClientMessage::Search {
            request_id,
            query,
            limit: 50,
        });
    });

    MiscCallbacks {
        on_stats,
        on_plugin_call,
        on_search,
    }
}
