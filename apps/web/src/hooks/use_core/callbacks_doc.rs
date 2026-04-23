use crate::api::WsService;
use crate::hooks::use_core::callbacks_scope::LocalScopeSignals;
use crate::hooks::use_core::navigation::PendingNavigation;
use crate::hooks::use_core::pending::PendingLocalEdits;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use deve_core::models::DocId;
use leptos::prelude::*;

#[path = "callbacks_doc_select.rs"]
mod select;
#[path = "callbacks_doc_write.rs"]
mod write;

use select::create_doc_select_callback;
use write::create_doc_write_callbacks;

pub struct DocCallbacks {
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
}

pub fn create_doc_callbacks(
    ws: &WsService,
    current_doc: ReadSignal<Option<DocId>>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    pending_local_edits: ReadSignal<PendingLocalEdits>,
    set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    set_current_doc: WriteSignal<Option<DocId>>,
    set_sync_banner: WriteSignal<Option<String>>,
    set_pending_created_doc_path: WriteSignal<Option<String>>,
    set_explicit_home: WriteSignal<bool>,
) -> DocCallbacks {
    let on_doc_select = create_doc_select_callback(
        current_doc,
        pending_local_edits,
        set_pending_navigation,
        set_current_doc,
        set_explicit_home,
    );
    let (on_doc_create, on_doc_rename, on_doc_delete, on_doc_copy, on_doc_move) =
        create_doc_write_callbacks(
            ws,
            current_doc,
            local_scope,
            write_gate,
            set_sync_banner,
            set_pending_created_doc_path,
            set_explicit_home,
        );
    DocCallbacks {
        on_doc_select,
        on_doc_create,
        on_doc_rename,
        on_doc_delete,
        on_doc_copy,
        on_doc_move,
    }
}
