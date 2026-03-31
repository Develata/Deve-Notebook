use crate::api::WsService;
use crate::hooks::use_core::callbacks_scope::LocalScopeSignals;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use leptos::prelude::*;

#[path = "callbacks_doc_write_create.rs"]
mod create;
#[path = "callbacks_doc_write_path.rs"]
mod path;
#[path = "callbacks_doc_write_scope.rs"]
mod scope;

use create::create_doc_create_callback;
use path::{
    create_doc_copy_callback, create_doc_delete_callback, create_doc_move_callback,
    create_doc_rename_callback,
};

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
    let on_doc_create = create_doc_create_callback(
        ws,
        current_doc,
        local_scope,
        write_gate,
        set_pending_created_doc_path,
        set_explicit_home,
    );
    let on_doc_rename = create_doc_rename_callback(ws, local_scope, write_gate);
    let on_doc_delete = create_doc_delete_callback(ws, local_scope, write_gate);
    let on_doc_copy = create_doc_copy_callback(ws, local_scope, write_gate);
    let on_doc_move = create_doc_move_callback(ws, local_scope, write_gate);
    (
        on_doc_create,
        on_doc_rename,
        on_doc_delete,
        on_doc_copy,
        on_doc_move,
    )
}
