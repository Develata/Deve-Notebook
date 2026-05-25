//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use deve_core::models::DocId;
use leptos::prelude::*;

mod read;
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

#[derive(Clone, Copy)]
pub struct SyncCallbackSignals {
    pub current_doc: ReadSignal<Option<DocId>>,
    pub local_scope: LocalScopeSignals,
    pub write_gate: RepoWriteSignals,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub set_shadow_list_request_id: WriteSignal<Option<String>>,
    pub set_sync_mode_request_id: WriteSignal<Option<String>>,
    pub set_pending_ops_request_id: WriteSignal<Option<String>>,
}

pub fn create_sync_callbacks(ws: &WsService, signals: SyncCallbackSignals) -> SyncCallbacks {
    let read = read::create_sync_read_callbacks(
        ws,
        signals.local_scope,
        signals.set_shadow_list_request_id,
        signals.set_sync_mode_request_id,
        signals.set_pending_ops_request_id,
        signals.set_sync_banner,
    );
    let write = write::create_sync_write_callbacks(
        ws,
        signals.current_doc,
        signals.local_scope,
        signals.write_gate,
        signals.set_sync_banner,
    );

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
