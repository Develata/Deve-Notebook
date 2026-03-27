use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::{to_target, to_targets};
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use deve_core::protocol::ClientMessage;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::Callback;

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

fn guarded_entry_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    action: &'static str,
    build: impl Fn(ChangeEntry, u64) -> ClientMessage + Clone + Send + Sync + 'static,
) -> Callback<ChangeEntry> {
    let ws = ws.clone();
    Callback::new(move |entry: ChangeEntry| {
        if let Some(label) = write_block_label(&ws, gate) {
            leptos::logging::warn!("忽略 {}: {}", action, label);
            return;
        }
        let build = build.clone();
        send_scoped(scope, &ws, move |scope_nonce| build(entry, scope_nonce));
    })
}

fn guarded_entries_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    action: &'static str,
    build: impl Fn(Vec<ChangeEntry>, u64) -> ClientMessage + Clone + Send + Sync + 'static,
) -> Callback<Vec<ChangeEntry>> {
    let ws = ws.clone();
    Callback::new(move |entries: Vec<ChangeEntry>| {
        if let Some(label) = write_block_label(&ws, gate) {
            leptos::logging::warn!("忽略 {}: {}", action, label);
            return;
        }
        let build = build.clone();
        send_scoped(scope, &ws, move |scope_nonce| build(entries, scope_nonce));
    })
}

pub(super) fn create_target_write_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
) -> (
    Callback<ChangeEntry>,
    Callback<Vec<ChangeEntry>>,
    Callback<ChangeEntry>,
    Callback<Vec<ChangeEntry>>,
    Callback<ChangeEntry>,
) {
    (
        guarded_entry_callback(ws, scope, gate, "StageFile", |entry, scope_nonce| {
            ClientMessage::StageFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            }
        }),
        guarded_entries_callback(ws, scope, gate, "StageFiles", |entries, scope_nonce| {
            ClientMessage::StageFiles {
                targets: to_targets(entries),
                scope_nonce: Some(scope_nonce),
            }
        }),
        guarded_entry_callback(ws, scope, gate, "UnstageFile", |entry, scope_nonce| {
            ClientMessage::UnstageFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            }
        }),
        guarded_entries_callback(ws, scope, gate, "UnstageFiles", |entries, scope_nonce| {
            ClientMessage::UnstageFiles {
                targets: to_targets(entries),
                scope_nonce: Some(scope_nonce),
            }
        }),
        guarded_entry_callback(ws, scope, gate, "DiscardFile", |entry, scope_nonce| {
            ClientMessage::DiscardFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            }
        }),
    )
}
