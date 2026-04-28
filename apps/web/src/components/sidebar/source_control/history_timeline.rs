//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!
use crate::components::sidebar::source_control::history_commit_item::HistoryCommitItem;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::i18n::Locale;
use deve_core::source_control::{CommitFileDiff, CommitInfo};
use leptos::prelude::*;

#[component]
pub fn HistoryTimeline(
    locale: RwSignal<Locale>,
    read_blocked: Signal<bool>,
    selected_commit: RwSignal<Option<String>>,
    compare_base_commit_id: RwSignal<Option<String>>,
    commit_history: ReadSignal<Vec<CommitInfo>>,
    commit_diff_request_id: ReadSignal<Option<String>>,
    set_commit_diff_request_id: WriteSignal<Option<String>>,
    commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    notice: ReadSignal<Option<SourceControlNotice>>,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
    set_commit_diff_result: WriteSignal<Vec<CommitFileDiff>>,
    on_get_commit_diff: Callback<(Option<String>, String)>,
) -> impl IntoView {
    view! {
        <div class="relative pl-6 pt-2">
            <div class="absolute left-[19px] top-2 bottom-0 w-[1px] bg-active"></div>
            <For
                each=move || commit_history.get()
                key=|c| c.id.clone()
                children=move |commit| {
                    view! {
                        <HistoryCommitItem
                            locale
                            read_blocked
                            selected_commit
                            compare_base_commit_id
                            commit
                            commit_diff_request_id
                            set_commit_diff_request_id
                            commit_diff_result
                            notice
                            set_notice
                            set_commit_diff_result
                            on_get_commit_diff
                        />
                    }
                }
            />
        </div>
    }
}
