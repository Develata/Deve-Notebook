//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
use deve_core::source_control::CommitFileDiff;
use leptos::prelude::*;

use super::super::diff_session::DiffSessionWire;
use super::super::source_control_notice::SourceControlNotice;

#[derive(Clone, Copy)]
pub(super) struct SourceControlSignals {
    pub staged_changes: ReadSignal<Vec<deve_core::source_control::ChangeEntry>>,
    pub set_staged_changes: WriteSignal<Vec<deve_core::source_control::ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<deve_core::source_control::ChangeEntry>>,
    pub set_unstaged_changes: WriteSignal<Vec<deve_core::source_control::ChangeEntry>>,
    pub confirmed_changes: ReadSignal<Vec<deve_core::source_control::ChangeEntry>>,
    pub set_confirmed_changes: WriteSignal<Vec<deve_core::source_control::ChangeEntry>>,
    pub changes_request_id: ReadSignal<Option<String>>,
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub commit_history: ReadSignal<Vec<deve_core::source_control::CommitInfo>>,
    pub set_commit_history: WriteSignal<Vec<deve_core::source_control::CommitInfo>>,
    pub commit_history_request_id: ReadSignal<Option<String>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
    pub doc_diff_request_id: ReadSignal<Option<String>>,
    pub set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub diff_content: ReadSignal<Option<DiffSessionWire>>,
    pub set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    pub commit_diff_request_id: ReadSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    pub set_commit_diff_result: WriteSignal<Vec<CommitFileDiff>>,
    pub source_control_notice: ReadSignal<Option<SourceControlNotice>>,
    pub set_source_control_notice: WriteSignal<Option<SourceControlNotice>>,
}

pub(super) fn init_source_control_signals() -> SourceControlSignals {
    let (staged_changes, set_staged_changes) = signal(Vec::new());
    let (unstaged_changes, set_unstaged_changes) = signal(Vec::new());
    let (confirmed_changes, set_confirmed_changes) = signal(Vec::new());
    let (changes_request_id, set_changes_request_id) = signal(None::<String>);
    let (commit_history, set_commit_history) = signal(Vec::new());
    let (commit_history_request_id, set_commit_history_request_id) = signal(None::<String>);
    let (doc_diff_request_id, set_doc_diff_request_id) = signal(None::<String>);
    let (diff_content, set_diff_content) = signal(None::<DiffSessionWire>);
    let (commit_diff_request_id, set_commit_diff_request_id) = signal(None::<String>);
    let (commit_diff_result, set_commit_diff_result) = signal(Vec::<CommitFileDiff>::new());
    let (source_control_notice, set_source_control_notice) = signal(None::<SourceControlNotice>);

    SourceControlSignals {
        staged_changes,
        set_staged_changes,
        unstaged_changes,
        set_unstaged_changes,
        confirmed_changes,
        set_confirmed_changes,
        changes_request_id,
        set_changes_request_id,
        commit_history,
        set_commit_history,
        commit_history_request_id,
        set_commit_history_request_id,
        doc_diff_request_id,
        set_doc_diff_request_id,
        diff_content,
        set_diff_content,
        commit_diff_request_id,
        set_commit_diff_request_id,
        commit_diff_result,
        set_commit_diff_result,
        source_control_notice,
        set_source_control_notice,
    }
}
