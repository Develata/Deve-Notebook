use crate::api::WsService;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use deve_core::source_control::{ChangeEntry, ConflictResolution};
use leptos::prelude::Callback;

use super::SourceControlScopeSignals;

#[path = "callbacks_sc_write_commit.rs"]
mod commit;
#[path = "callbacks_sc_write_targets.rs"]
mod targets;
#[path = "callbacks_sc_write_targets_guard.rs"]
mod targets_guard;

use commit::create_commit_write_callbacks;
use targets::create_target_write_callbacks;

pub(super) fn create_write_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
) -> (
    Callback<ChangeEntry>,
    Callback<Vec<ChangeEntry>>,
    Callback<ChangeEntry>,
    Callback<Vec<ChangeEntry>>,
    Callback<ChangeEntry>,
    Callback<String>,
    Callback<(ChangeEntry, ConflictResolution)>,
    Callback<String>,
) {
    let (on_stage_file, on_stage_files, on_unstage_file, on_unstage_files, on_discard_file) =
        create_target_write_callbacks(ws, scope, gate);
    let (on_commit, on_resolve_conflict, on_commit_and_push) =
        create_commit_write_callbacks(ws, scope, gate);
    (
        on_stage_file,
        on_stage_files,
        on_unstage_file,
        on_unstage_files,
        on_discard_file,
        on_commit,
        on_resolve_conflict,
        on_commit_and_push,
    )
}
