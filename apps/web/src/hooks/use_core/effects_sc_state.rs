use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

pub(crate) struct ScStateResetSignals {
    pub set_staged: WriteSignal<Vec<ChangeEntry>>,
    pub set_unstaged: WriteSignal<Vec<ChangeEntry>>,
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub set_history: WriteSignal<Vec<CommitInfo>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
    pub set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub set_diff: WriteSignal<Option<DiffSessionWire>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub set_commit_diff: WriteSignal<Vec<CommitFileDiff>>,
}

pub(crate) fn clear_repo_scoped_state(signals: ScStateResetSignals) {
    signals.set_staged.set(Vec::new());
    signals.set_unstaged.set(Vec::new());
    signals.set_changes_request_id.set(None);
    signals.set_history.set(Vec::new());
    signals.set_commit_history_request_id.set(None);
    signals.set_doc_diff_request_id.set(None);
    signals.set_diff.set(None);
    signals.set_commit_diff_request_id.set(None);
    signals.set_commit_diff.set(Vec::new());
}

pub(crate) fn doc_diff_matches_request(
    request_id: &Option<String>,
    expected_request_id: Option<String>,
) -> bool {
    request_id
        .as_deref()
        .is_none_or(|request_id| expected_request_id.as_deref() == Some(request_id))
}

pub(crate) fn changes_list_matches_request(
    request_id: &Option<String>,
    expected_request_id: Option<String>,
) -> bool {
    match request_id {
        Some(request_id) => expected_request_id.as_deref() == Some(request_id.as_str()),
        None => true,
    }
}

pub(crate) fn commit_history_matches_request(
    request_id: &Option<String>,
    expected_request_id: Option<String>,
) -> bool {
    request_id
        .as_deref()
        .is_some_and(|request_id| expected_request_id.as_deref() == Some(request_id))
}

pub(crate) fn commit_diff_matches_request(
    request_id: &Option<String>,
    expected_request_id: Option<String>,
) -> bool {
    request_id
        .as_deref()
        .is_some_and(|request_id| expected_request_id.as_deref() == Some(request_id))
}
