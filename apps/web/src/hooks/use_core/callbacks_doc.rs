//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime
//!
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

#[derive(Clone, Copy)]
pub struct DocCallbackSignals {
    pub current_doc: ReadSignal<Option<DocId>>,
    pub local_scope: LocalScopeSignals,
    pub write_gate: RepoWriteSignals,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub set_pending_created_doc_path: WriteSignal<Option<String>>,
    pub set_explicit_home: WriteSignal<bool>,
}

pub fn create_doc_callbacks(ws: &WsService, signals: DocCallbackSignals) -> DocCallbacks {
    let on_doc_select = create_doc_select_callback(
        signals.current_doc,
        signals.local_scope.current_repo_id,
        signals.local_scope.current_scope_nonce,
        signals.pending_local_edits,
        signals.set_pending_navigation,
        signals.set_current_doc,
        signals.set_explicit_home,
    );
    let write = create_doc_write_callbacks(
        ws,
        signals.current_doc,
        signals.local_scope,
        signals.write_gate,
        signals.set_sync_banner,
        signals.set_pending_created_doc_path,
        signals.set_explicit_home,
    );
    DocCallbacks {
        on_doc_select,
        on_doc_create: write.on_doc_create,
        on_doc_rename: write.on_doc_rename,
        on_doc_delete: write.on_doc_delete,
        on_doc_copy: write.on_doc_copy,
        on_doc_move: write.on_doc_move,
    }
}
