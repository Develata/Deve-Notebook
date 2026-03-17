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

use super::PendingBranchTarget;
use super::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
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
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 CreateDoc: local repo scope 尚未稳定");
            return;
        };
        ws_for_create.send(ClientMessage::CreateDoc {
            name,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_for_rename = ws.clone();
    let on_doc_rename = Callback::new(move |(old_path, new_path): (String, String)| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 RenameDoc: local repo scope 尚未稳定");
            return;
        };
        leptos::logging::log!("重命名: {} -> {}", old_path, new_path);
        ws_for_rename.send(ClientMessage::RenameDoc {
            old_path,
            new_path,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_for_delete = ws.clone();
    let on_doc_delete = Callback::new(move |path: String| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 DeleteDoc: local repo scope 尚未稳定");
            return;
        };
        leptos::logging::log!("删除: {}", path);
        ws_for_delete.send(ClientMessage::DeleteDoc {
            path,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_for_copy = ws.clone();
    let on_doc_copy = Callback::new(move |(src_path, dest_path): (String, String)| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 CopyDoc: local repo scope 尚未稳定");
            return;
        };
        leptos::logging::log!("复制: {} -> {}", src_path, dest_path);
        ws_for_copy.send(ClientMessage::CopyDoc {
            src_path,
            dest_path,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_for_move = ws.clone();
    let on_doc_move = Callback::new(move |(src_path, dest_path): (String, String)| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 MoveDoc: local repo scope 尚未稳定");
            return;
        };
        leptos::logging::log!("移动: {} -> {}", src_path, dest_path);
        ws_for_move.send(ClientMessage::MoveDoc {
            src_path,
            dest_path,
            scope_nonce: Some(scope_nonce),
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

pub struct SearchScopeSignals {
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
}

pub struct MiscRequestSignals {
    pub set_plugin_request_ids: WriteSignal<Vec<String>>,
    pub set_search_request_id: WriteSignal<Option<String>>,
}

/// 创建其他回调
pub fn create_misc_callbacks(
    ws: &WsService,
    set_stats: WriteSignal<crate::editor::EditorStats>,
    load_state: ReadSignal<String>,
    search_scope: SearchScopeSignals,
    request_signals: MiscRequestSignals,
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
            request_signals.set_plugin_request_ids.update(|ids| {
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
        if search_scope.pending_branch_switch.get_untracked().is_some()
            || search_scope.pending_repo_switch.get_untracked().is_some()
        {
            leptos::logging::warn!("Search disabled while scope switch is pending");
            return;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        request_signals
            .set_search_request_id
            .set(Some(request_id.clone()));
        ws_search.send(ClientMessage::Search {
            request_id,
            query,
            limit: 50,
            scope_nonce: Some(search_scope.current_scope_nonce.get_untracked()),
        });
    });

    MiscCallbacks {
        on_stats,
        on_plugin_call,
        on_search,
    }
}
