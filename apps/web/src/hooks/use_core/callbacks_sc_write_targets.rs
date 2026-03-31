use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_target::{to_target, to_targets};
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use deve_core::protocol::ClientMessage;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::Callback;

use super::SourceControlScopeSignals;
use super::targets_guard::{guarded_entries_callback, guarded_entry_callback};

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
