use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::cannot_send;
use deve_core::protocol::ClientMessage;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::{Callback, WriteSignal};

use super::SourceControlScopeSignals;

fn send_scoped(
    scope: SourceControlScopeSignals,
    ws: &WsService,
    build: impl FnOnce(u64) -> ClientMessage,
) {
    let Some(scope_nonce) = source_control_scope_nonce(scope) else {
        return;
    };
    ws.send(build(scope_nonce));
}

fn write_block_label(ws: &WsService, gate: RepoWriteSignals) -> Option<&'static str> {
    repo_write_block_untracked(ws, gate).map(|block| block.label())
}

pub(super) fn guarded_entry_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    action: &'static str,
    build: impl Fn(ChangeEntry, u64) -> ClientMessage + Clone + Send + Sync + 'static,
) -> Callback<ChangeEntry> {
    let ws = ws.clone();
    Callback::new(move |entry: ChangeEntry| {
        if let Some(label) = write_block_label(&ws, gate) {
            let message = cannot_send(action, label);
            warn_sync_banner(set_sync_banner, message);
            return;
        }
        let build = build.clone();
        send_scoped(scope, &ws, move |scope_nonce| build(entry, scope_nonce));
    })
}

pub(super) fn guarded_entries_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    action: &'static str,
    build: impl Fn(Vec<ChangeEntry>, u64) -> ClientMessage + Clone + Send + Sync + 'static,
) -> Callback<Vec<ChangeEntry>> {
    let ws = ws.clone();
    Callback::new(move |entries: Vec<ChangeEntry>| {
        if let Some(label) = write_block_label(&ws, gate) {
            let message = cannot_send(action, label);
            warn_sync_banner(set_sync_banner, message);
            return;
        }
        let build = build.clone();
        send_scoped(scope, &ws, move |scope_nonce| build(entries, scope_nonce));
    })
}
