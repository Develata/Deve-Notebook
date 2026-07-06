//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use crate::i18n::Locale;
use crate::runtime::scope_client::LocalScopeSignals;
use leptos::prelude::*;

mod create;
mod path;
mod scope;

use create::create_doc_create_callback;
use path::{
    create_doc_copy_callback, create_doc_delete_callback, create_doc_move_callback,
    create_doc_rename_callback,
};

pub(super) struct DocWriteCallbacks {
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
}

#[derive(Clone, Copy)]
pub(super) struct DocWriteSignals {
    pub locale: RwSignal<Locale>,
    pub current_doc: ReadSignal<Option<deve_core::models::DocId>>,
    pub local_scope: LocalScopeSignals,
    pub write_gate: RepoWriteSignals,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub set_pending_created_doc_path: WriteSignal<Option<String>>,
    pub set_explicit_home: WriteSignal<bool>,
}

pub(super) fn create_doc_write_callbacks(
    ws: &WsService,
    signals: DocWriteSignals,
) -> DocWriteCallbacks {
    let on_doc_create = create_doc_create_callback(ws, signals);
    let on_doc_rename = create_doc_rename_callback(
        ws,
        signals.locale,
        signals.local_scope,
        signals.write_gate,
        signals.set_sync_banner,
    );
    let on_doc_delete = create_doc_delete_callback(
        ws,
        signals.locale,
        signals.local_scope,
        signals.write_gate,
        signals.set_sync_banner,
    );
    let on_doc_copy = create_doc_copy_callback(
        ws,
        signals.locale,
        signals.local_scope,
        signals.write_gate,
        signals.set_sync_banner,
    );
    let on_doc_move = create_doc_move_callback(
        ws,
        signals.locale,
        signals.local_scope,
        signals.write_gate,
        signals.set_sync_banner,
    );
    DocWriteCallbacks {
        on_doc_create,
        on_doc_rename,
        on_doc_delete,
        on_doc_copy,
        on_doc_move,
    }
}
