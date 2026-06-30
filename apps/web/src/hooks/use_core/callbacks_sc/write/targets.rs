//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_target::{to_target, to_targets};
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use crate::hooks::use_core::write_gate_banner::WriteGateAction;
use crate::i18n::Locale;
use deve_core::protocol::ClientMessage;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::{Callback, RwSignal, WriteSignal};

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
    locale: RwSignal<Locale>,
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
            locale,
            WriteGateAction::StageFile,
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
            locale,
            WriteGateAction::StageFiles,
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
            locale,
            WriteGateAction::UnstageFile,
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
            locale,
            WriteGateAction::UnstageFiles,
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
            locale,
            WriteGateAction::DiscardFile,
            |entry, scope_nonce| ClientMessage::DiscardFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            },
        ),
    )
}
