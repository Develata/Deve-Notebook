//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_target::{to_target, to_targets};
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use deve_core::protocol::ClientMessage;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::{Callback, WriteSignal};

use super::SourceControlScopeSignals;
use super::targets_guard::{guarded_entries_callback, guarded_entry_callback};

type SourceControlTargetWriteCallbacks = (
    Callback<ChangeEntry>,
    Callback<Vec<ChangeEntry>>,
    Callback<ChangeEntry>,
    Callback<Vec<ChangeEntry>>,
    Callback<ChangeEntry>,
);

pub(super) fn create_target_write_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> SourceControlTargetWriteCallbacks {
    (
        guarded_entry_callback(
            ws,
            scope,
            gate,
            set_sync_banner,
            "StageFile",
            |entry, scope_nonce| ClientMessage::StageFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            },
        ),
        guarded_entries_callback(
            ws,
            scope,
            gate,
            set_sync_banner,
            "StageFiles",
            |entries, scope_nonce| ClientMessage::StageFiles {
                targets: to_targets(entries),
                scope_nonce: Some(scope_nonce),
            },
        ),
        guarded_entry_callback(
            ws,
            scope,
            gate,
            set_sync_banner,
            "UnstageFile",
            |entry, scope_nonce| ClientMessage::UnstageFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            },
        ),
        guarded_entries_callback(
            ws,
            scope,
            gate,
            set_sync_banner,
            "UnstageFiles",
            |entries, scope_nonce| ClientMessage::UnstageFiles {
                targets: to_targets(entries),
                scope_nonce: Some(scope_nonce),
            },
        ),
        guarded_entry_callback(
            ws,
            scope,
            gate,
            set_sync_banner,
            "DiscardFile",
            |entry, scope_nonce| ClientMessage::DiscardFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            },
        ),
    )
}
