//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use crate::hooks::use_core::write_gate_banner::WriteGateAction;
use crate::i18n::Locale;
use crate::runtime::scope_client::LocalScopeSignals;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::scope::local_write_scope_nonce;

pub(super) fn create_doc_rename_callback(
    ws: &WsService,
    locale: RwSignal<Locale>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<(String, String)> {
    let ws = ws.clone();
    Callback::new(move |(old_path, new_path)| {
        let Some(scope_nonce) = local_write_scope_nonce(
            &ws,
            locale,
            local_scope,
            write_gate,
            set_sync_banner,
            WriteGateAction::RenameDoc,
        ) else {
            return;
        };
        leptos::logging::log!("重命名: {} -> {}", old_path, new_path);
        ws.send(ClientMessage::RenameDoc {
            old_path,
            new_path,
            scope_nonce: Some(scope_nonce),
        });
    })
}

pub(super) fn create_doc_delete_callback(
    ws: &WsService,
    locale: RwSignal<Locale>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<String> {
    let ws = ws.clone();
    Callback::new(move |path: String| {
        let Some(scope_nonce) = local_write_scope_nonce(
            &ws,
            locale,
            local_scope,
            write_gate,
            set_sync_banner,
            WriteGateAction::DeleteDoc,
        ) else {
            return;
        };
        leptos::logging::log!("删除: {}", path);
        ws.send(ClientMessage::DeleteDoc {
            path,
            scope_nonce: Some(scope_nonce),
        });
    })
}

pub(super) fn create_doc_copy_callback(
    ws: &WsService,
    locale: RwSignal<Locale>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<(String, String)> {
    let ws = ws.clone();
    Callback::new(move |(src_path, dest_path)| {
        let Some(scope_nonce) = local_write_scope_nonce(
            &ws,
            locale,
            local_scope,
            write_gate,
            set_sync_banner,
            WriteGateAction::CopyDoc,
        ) else {
            return;
        };
        leptos::logging::log!("复制: {} -> {}", src_path, dest_path);
        ws.send(ClientMessage::CopyDoc {
            src_path,
            dest_path,
            scope_nonce: Some(scope_nonce),
        });
    })
}

pub(super) fn create_doc_move_callback(
    ws: &WsService,
    locale: RwSignal<Locale>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<(String, String)> {
    let ws = ws.clone();
    Callback::new(move |(src_path, dest_path)| {
        let Some(scope_nonce) = local_write_scope_nonce(
            &ws,
            locale,
            local_scope,
            write_gate,
            set_sync_banner,
            WriteGateAction::MoveDoc,
        ) else {
            return;
        };
        leptos::logging::log!("移动: {} -> {}", src_path, dest_path);
        ws.send(ClientMessage::MoveDoc {
            src_path,
            dest_path,
            scope_nonce: Some(scope_nonce),
        });
    })
}
