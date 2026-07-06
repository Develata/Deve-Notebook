//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, ConflictResolution};
use leptos::prelude::*;

use super::super::callbacks_sc::SourceControlCallbacks;
use super::super::source_control_notice::SourceControlNotice;
use super::super::state::CoreSignals;
use crate::runtime::source_control_client::diff_session::DiffSessionWire;

pub(super) struct SourceControlStateSection {
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub confirmed_changes: ReadSignal<Vec<ChangeEntry>>,
    pub set_confirmed_changes: WriteSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub commit_history_request_id: ReadSignal<Option<String>>,
    pub commit_diff_request_id: ReadSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub diff_content: ReadSignal<Option<DiffSessionWire>>,
    pub set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    pub on_get_doc_diff: Callback<ChangeEntry>,
    pub commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    pub set_commit_diff_result: WriteSignal<Vec<CommitFileDiff>>,
    pub source_control_notice: ReadSignal<Option<SourceControlNotice>>,
    pub set_source_control_notice: WriteSignal<Option<SourceControlNotice>>,
    pub on_resolve_conflict: Callback<(ChangeEntry, ConflictResolution)>,
    pub on_get_commit_diff: Callback<(Option<String>, String)>,
    pub on_commit_and_push: Callback<String>,
}

pub(super) fn build_source_control_section(
    signals: &CoreSignals,
    sc: &SourceControlCallbacks,
) -> SourceControlStateSection {
    SourceControlStateSection {
        staged_changes: signals.staged_changes,
        unstaged_changes: signals.unstaged_changes,
        confirmed_changes: signals.confirmed_changes,
        set_confirmed_changes: signals.set_confirmed_changes,
        commit_history: signals.commit_history,
        commit_history_request_id: signals.commit_history_request_id,
        commit_diff_request_id: signals.commit_diff_request_id,
        set_commit_diff_request_id: signals.set_commit_diff_request_id,
        on_get_changes: sc.on_get_changes,
        on_stage_file: sc.on_stage_file,
        on_stage_files: sc.on_stage_files,
        on_unstage_file: sc.on_unstage_file,
        on_unstage_files: sc.on_unstage_files,
        on_discard_file: sc.on_discard_file,
        on_commit: sc.on_commit,
        on_get_history: sc.on_get_history,
        diff_content: signals.diff_content,
        set_diff_content: signals.set_diff_content,
        on_get_doc_diff: sc.on_get_doc_diff,
        commit_diff_result: signals.commit_diff_result,
        set_commit_diff_result: signals.set_commit_diff_result,
        source_control_notice: signals.source_control_notice,
        set_source_control_notice: signals.set_source_control_notice,
        on_resolve_conflict: sc.on_resolve_conflict,
        on_get_commit_diff: sc.on_get_commit_diff,
        on_commit_and_push: sc.on_commit_and_push,
    }
}
