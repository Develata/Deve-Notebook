use crate::api::WsService;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::{Callback, WriteSignal};

#[path = "callbacks_sc_read_changes.rs"]
mod changes;
#[path = "callbacks_sc_read_diff.rs"]
mod diff;

use super::{SourceControlRequestSignals, SourceControlScopeSignals};
use changes::{create_get_changes_callback, create_get_history_callback};
use diff::{create_get_commit_diff_callback, create_get_doc_diff_callback};

type SourceControlReadCallbacks = (
    Callback<()>,
    Callback<u32>,
    Callback<ChangeEntry>,
    Callback<(Option<String>, String)>,
);

pub(super) fn log_blocked_sc_read(
    action: &str,
    detail: &str,
    block: crate::hooks::use_core::write_gate::RepoWriteBlock,
) {
    leptos::logging::log!(
        "跳过 {}: source control blocked by {} for {}",
        action,
        block.label(),
        detail
    );
}

pub(super) fn create_read_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    request: SourceControlRequestSignals,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
    set_diff_content: WriteSignal<Option<DiffSessionWire>>,
) -> SourceControlReadCallbacks {
    (
        create_get_changes_callback(ws, scope, request.set_changes_request_id),
        create_get_history_callback(ws, scope, read_gate, request.set_commit_history_request_id),
        create_get_doc_diff_callback(
            ws,
            scope,
            read_gate,
            request.set_doc_diff_request_id,
            set_notice,
            set_diff_content,
        ),
        create_get_commit_diff_callback(ws, scope, read_gate, request.set_commit_diff_request_id),
    )
}
