//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use crate::hooks::use_core::{PendingBranchSwitch, PendingRepoSwitch};
use deve_core::source_control::{ChangeEntry, ConflictResolution};
use leptos::prelude::*;

mod read;
mod write;

use read::create_read_callbacks;
use write::create_write_callbacks;

pub struct SourceControlCallbacks {
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub on_get_doc_diff: Callback<ChangeEntry>,
    pub on_resolve_conflict: Callback<(ChangeEntry, ConflictResolution)>,
    pub on_get_commit_diff: Callback<(Option<String>, String)>,
    pub on_commit_and_push: Callback<String>,
}

#[derive(Clone, Copy)]
pub struct SourceControlScopeSignals {
    pub current_repo_id: ReadSignal<Option<String>>,
    pub active_branch: ReadSignal<Option<deve_core::models::PeerId>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
}

#[derive(Clone, Copy)]
pub struct SourceControlRequestSignals {
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
    pub set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
}

pub fn create_source_control_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    write_gate: RepoWriteSignals,
    request: SourceControlRequestSignals,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
    set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    set_sync_banner: WriteSignal<Option<String>>,
) -> SourceControlCallbacks {
    let (on_get_changes, on_get_history, on_get_doc_diff, on_get_commit_diff) =
        create_read_callbacks(ws, scope, write_gate, request, set_notice, set_diff_content);
    let (
        on_stage_file,
        on_stage_files,
        on_unstage_file,
        on_unstage_files,
        on_discard_file,
        on_commit,
        on_resolve_conflict,
        on_commit_and_push,
    ) = create_write_callbacks(ws, scope, write_gate, set_sync_banner);

    SourceControlCallbacks {
        on_get_changes,
        on_stage_file,
        on_stage_files,
        on_unstage_file,
        on_unstage_files,
        on_discard_file,
        on_commit,
        on_get_history,
        on_get_doc_diff,
        on_resolve_conflict,
        on_get_commit_diff,
        on_commit_and_push,
    }
}
