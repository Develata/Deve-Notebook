use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use deve_core::source_control::CommitFileDiffSummary;
use leptos::prelude::*;

pub fn reset_compare_state(
    selected_commit: RwSignal<Option<String>>,
    compare_base_commit_id: RwSignal<Option<String>>,
    set_commit_diff_request_id: WriteSignal<Option<String>>,
    set_commit_diff_result: WriteSignal<Vec<CommitFileDiffSummary>>,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
) {
    compare_base_commit_id.set(None);
    selected_commit.set(None);
    set_commit_diff_request_id.set(None);
    set_commit_diff_result.set(Vec::new());
    set_notice.set(None);
}
