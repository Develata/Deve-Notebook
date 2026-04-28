use crate::components::sidebar::source_control::history_diff_row::HistoryDiffRow;
use crate::components::sidebar::source_control::history_empty_state::no_diff_message;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::i18n::{Locale, t};
use deve_core::source_control::CommitFileDiff;
use leptos::prelude::*;

#[component]
pub fn HistoryCommitDetails(
    locale: RwSignal<Locale>,
    compare_base_commit_id: RwSignal<Option<String>>,
    target_commit_id: String,
    commit_diff_request_id: ReadSignal<Option<String>>,
    commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    notice: ReadSignal<Option<SourceControlNotice>>,
) -> impl IntoView {
    view! {
        {move || {
            if commit_diff_request_id.get().is_some() {
                view! {
                    <div class="py-1 text-[12px] text-muted">
                        {move || t::source_control::loading_commit_diff(locale.get())}
                    </div>
                }
                .into_any()
            } else if !commit_diff_result.get().is_empty() {
                view! {
                    {commit_diff_result.get().into_iter().map(|file| view! { <HistoryDiffRow file /> }).collect_view()}
                }
                .into_any()
            } else if notice.get().is_none() {
                let empty_message = no_diff_message(
                    locale.get(),
                    compare_base_commit_id.get().as_deref(),
                    target_commit_id.as_str(),
                );
                view! {
                    <div class="py-1 text-[12px] text-muted">{empty_message}</div>
                }
                .into_any()
            } else {
                view! {}.into_any()
            }
        }}
    }
}
