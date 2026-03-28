use crate::api::WsService;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use deve_core::models::DocId;
use leptos::prelude::*;

#[path = "callbacks_sync_read.rs"]
mod read;
#[path = "callbacks_sync_write.rs"]
mod write;

use super::callbacks_scope::LocalScopeSignals;

pub struct SyncCallbacks {
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    pub on_list_shadows: Callback<()>,
    pub on_merge_peer: Callback<String>,
}

pub fn create_sync_callbacks(
    ws: &WsService,
    current_doc: ReadSignal<Option<DocId>>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_shadow_list_request_id: WriteSignal<Option<String>>,
    set_sync_mode_request_id: WriteSignal<Option<String>>,
    set_pending_ops_request_id: WriteSignal<Option<String>>,
) -> SyncCallbacks {
    let read = read::create_sync_read_callbacks(
        ws,
        local_scope,
        set_shadow_list_request_id,
        set_sync_mode_request_id,
        set_pending_ops_request_id,
    );
    let write = write::create_sync_write_callbacks(ws, current_doc, local_scope, write_gate);

    SyncCallbacks {
        on_get_sync_mode: read.on_get_sync_mode,
        on_set_sync_mode: write.on_set_sync_mode,
        on_get_pending_ops: read.on_get_pending_ops,
        on_confirm_merge: write.on_confirm_merge,
        on_discard_pending: write.on_discard_pending,
        on_list_shadows: read.on_list_shadows,
        on_merge_peer: write.on_merge_peer,
    }
}
