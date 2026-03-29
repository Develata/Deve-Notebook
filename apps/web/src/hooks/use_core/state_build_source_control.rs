use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, ConflictResolution};
use leptos::prelude::*;

use super::super::callbacks_sc::SourceControlCallbacks;
use super::super::diff_session::DiffSessionWire;
use super::super::state::CoreSignals;

pub(super) struct SourceControlStateSection {
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
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
        commit_history: signals.commit_history,
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
        on_resolve_conflict: sc.on_resolve_conflict,
        on_get_commit_diff: sc.on_get_commit_diff,
        on_commit_and_push: sc.on_commit_and_push,
    }
}
