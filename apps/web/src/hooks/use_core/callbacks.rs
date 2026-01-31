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

// Re-export from submodule
#[allow(unused_imports)] // SourceControlCallbacks 为外部模块预留
pub use super::callbacks_sc::{create_source_control_callbacks, SourceControlCallbacks};

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
    set_current_doc: WriteSignal<Option<DocId>>,
) -> DocCallbacks {
    let on_doc_select = Callback::new(move |id: DocId| {
        set_current_doc.set(Some(id));
    });

    let ws_for_create = ws.clone();
    let on_doc_create = Callback::new(move |name: String| {
        ws_for_create.send(ClientMessage::CreateDoc { name });
    });

    let ws_for_rename = ws.clone();
    let on_doc_rename = Callback::new(move |(old_path, new_path): (String, String)| {
        leptos::logging::log!("重命名: {} -> {}", old_path, new_path);
        ws_for_rename.send(ClientMessage::RenameDoc { old_path, new_path });
    });

    let ws_for_delete = ws.clone();
    let on_doc_delete = Callback::new(move |path: String| {
        leptos::logging::log!("删除: {}", path);
        ws_for_delete.send(ClientMessage::DeleteDoc { path });
    });

    let ws_for_copy = ws.clone();
    let on_doc_copy = Callback::new(move |(src_path, dest_path): (String, String)| {
        leptos::logging::log!("复制: {} -> {}", src_path, dest_path);
        ws_for_copy.send(ClientMessage::CopyDoc {
            src_path,
            dest_path,
        });
    });

    let ws_for_move = ws.clone();
    let on_doc_move = Callback::new(move |(src_path, dest_path): (String, String)| {
        leptos::logging::log!("移动: {} -> {}", src_path, dest_path);
        ws_for_move.send(ClientMessage::MoveDoc {
            src_path,
            dest_path,
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

/// 同步/合并操作回调
pub struct SyncCallbacks {
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    pub on_list_shadows: Callback<()>,
    pub on_merge_peer: Callback<String>,
}

/// 创建同步回调
pub fn create_sync_callbacks(
    ws: &WsService,
    current_doc: ReadSignal<Option<DocId>>,
) -> SyncCallbacks {
    let ws1 = ws.clone();
    let on_get_sync_mode = Callback::new(move |_: ()| {
        ws1.send(ClientMessage::GetSyncMode);
    });

    let ws2 = ws.clone();
    let on_set_sync_mode = Callback::new(move |mode: String| {
        ws2.send(ClientMessage::SetSyncMode { mode });
    });

    let ws3 = ws.clone();
    let on_get_pending_ops = Callback::new(move |_: ()| {
        ws3.send(ClientMessage::GetPendingOps);
    });

    let ws4 = ws.clone();
    let on_confirm_merge = Callback::new(move |_: ()| {
        ws4.send(ClientMessage::ConfirmMerge);
    });

    let ws5 = ws.clone();
    let on_discard_pending = Callback::new(move |_: ()| {
        ws5.send(ClientMessage::DiscardPending);
    });

    let ws6 = ws.clone();
    let on_list_shadows = Callback::new(move |_: ()| {
        ws6.send(ClientMessage::ListShadows);
    });

    let ws7 = ws.clone();
    let on_merge_peer = Callback::new(move |peer_id: String| {
        if let Some(doc_id) = current_doc.get_untracked() {
            ws7.send(ClientMessage::MergePeer { peer_id, doc_id });
        }
    });

    SyncCallbacks {
        on_get_sync_mode,
        on_set_sync_mode,
        on_get_pending_ops,
        on_confirm_merge,
        on_discard_pending,
        on_list_shadows,
        on_merge_peer,
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
        ws_search.send(ClientMessage::Search { query, limit: 50 });
    });

    MiscCallbacks {
        on_stats,
        on_plugin_call,
        on_search,
    }
}

/// 切换操作回调
pub struct SwitchCallbacks {
    pub on_switch_branch: Callback<Option<String>>,
    pub on_switch_repo: Callback<String>,
}

/// 创建切换回调
pub fn create_switch_callbacks(ws: &WsService) -> SwitchCallbacks {
    let ws_branch = ws.clone();
    let on_switch_branch = Callback::new(move |peer_id: Option<String>| {
        leptos::logging::log!("触发 SwitchBranch 回调: {:?}", peer_id);
        ws_branch.send(ClientMessage::SwitchBranch { peer_id });
    });

    let ws_repo = ws.clone();
    let on_switch_repo = Callback::new(move |name: String| {
        leptos::logging::log!("触发 SwitchRepo 回调: {}", name);
        ws_repo.send(ClientMessage::SwitchRepo { name });
    });

    SwitchCallbacks {
        on_switch_branch,
        on_switch_repo,
    }
}
