use crate::api::WsService;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

fn local_write_scope_nonce(
    ws: &WsService,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    action: &'static str,
) -> Option<u64> {
    if let Some(block) = repo_write_block_untracked(ws, write_gate) {
        leptos::logging::warn!("忽略 {}: {}", action, block.label());
        return None;
    }
    let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
        leptos::logging::warn!("忽略 {}: local repo scope 尚未稳定", action);
        return None;
    };
    Some(scope_nonce)
}

pub(super) fn create_doc_write_callbacks(
    ws: &WsService,
    current_doc: ReadSignal<Option<deve_core::models::DocId>>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_pending_created_doc_path: WriteSignal<Option<String>>,
    set_explicit_home: WriteSignal<bool>,
) -> (
    Callback<String>,
    Callback<(String, String)>,
    Callback<String>,
    Callback<(String, String)>,
    Callback<(String, String)>,
) {
    let ws_for_create = ws.clone();
    let on_doc_create = Callback::new(move |name: String| {
        let Some(scope_nonce) =
            local_write_scope_nonce(&ws_for_create, local_scope, write_gate, "CreateDoc")
        else {
            return;
        };
        if current_doc.get_untracked().is_none() {
            set_explicit_home.set(false);
            set_pending_created_doc_path.set(Some(name.clone()));
        } else {
            set_pending_created_doc_path.set(None);
        }
        ws_for_create.send(ClientMessage::CreateDoc {
            name,
            scope_nonce: Some(scope_nonce),
        });
    });
    let ws_for_rename = ws.clone();
    let on_doc_rename = Callback::new(move |(old_path, new_path)| {
        let Some(scope_nonce) =
            local_write_scope_nonce(&ws_for_rename, local_scope, write_gate, "RenameDoc")
        else {
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
        let Some(scope_nonce) =
            local_write_scope_nonce(&ws_for_delete, local_scope, write_gate, "DeleteDoc")
        else {
            return;
        };
        leptos::logging::log!("删除: {}", path);
        ws_for_delete.send(ClientMessage::DeleteDoc {
            path,
            scope_nonce: Some(scope_nonce),
        });
    });
    let ws_for_copy = ws.clone();
    let on_doc_copy = Callback::new(move |(src_path, dest_path)| {
        let Some(scope_nonce) =
            local_write_scope_nonce(&ws_for_copy, local_scope, write_gate, "CopyDoc")
        else {
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
    let on_doc_move = Callback::new(move |(src_path, dest_path)| {
        let Some(scope_nonce) =
            local_write_scope_nonce(&ws_for_move, local_scope, write_gate, "MoveDoc")
        else {
            return;
        };
        leptos::logging::log!("移动: {} -> {}", src_path, dest_path);
        ws_for_move.send(ClientMessage::MoveDoc {
            src_path,
            dest_path,
            scope_nonce: Some(scope_nonce),
        });
    });
    (
        on_doc_create,
        on_doc_rename,
        on_doc_delete,
        on_doc_copy,
        on_doc_move,
    )
}
