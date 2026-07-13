//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
use crate::components::sidebar::source_control::history_commit_details::HistoryCommitDetails;
use crate::components::sidebar::source_control::history_commit_state::{
    HistoryCommitVisualState, history_commit_row_class, resolve_history_commit_visual_state,
};
use crate::components::sidebar::source_control::history_compare_logic::{
    HistorySelectionAction, resolve_history_selection, short_commit_id,
};
use crate::hooks::use_core::SourceControlContext;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::i18n::{Locale, source_control};
use crate::utils::time::format_relative;
use deve_core::source_control::{CommitFileDiffSummary, CommitInfo};
use leptos::prelude::*;

#[component]
pub fn HistoryCommitItem(
    locale: RwSignal<Locale>,
    read_blocked: Signal<bool>,
    selected_commit: RwSignal<Option<String>>,
    compare_base_commit_id: RwSignal<Option<String>>,
    commit: CommitInfo,
    commit_diff_request_id: ReadSignal<Option<String>>,
    set_commit_diff_request_id: WriteSignal<Option<String>>,
    commit_diff_result: ReadSignal<Vec<CommitFileDiffSummary>>,
    notice: ReadSignal<Option<SourceControlNotice>>,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
    set_commit_diff_result: WriteSignal<Vec<CommitFileDiffSummary>>,
    on_get_commit_diff: Callback<(Option<String>, String)>,
) -> impl IntoView {
    let source_control = expect_context::<SourceControlContext>();
    let commit_id = commit.id.clone();
    let selected_commit_id = commit_id.clone();
    let commit_for_click = commit.clone();
    let parent_commit_id = commit.parent_id.clone();
    let visual_state = Signal::derive(move || {
        resolve_history_commit_visual_state(
            selected_commit.get().as_deref(),
            compare_base_commit_id.get().as_deref(),
            commit_id.as_str(),
        )
    });

    view! {
        <div class="relative mb-3 group">
            <div class="absolute -left-[19px] top-[3px] w-2.5 h-2.5 rounded-full border-2 border-white bg-accent shadow-sm z-[calc(var(--z-editor)_+_1)]"></div>
            <div
                class=move || history_commit_row_class(visual_state.get(), read_blocked.get())
                on:click=move |_| {
                    if read_blocked.get_untracked() {
                        return;
                    }
                    source_control.set_diff_content.set(None);
                    match resolve_history_selection(
                        selected_commit.get_untracked().as_deref(),
                        compare_base_commit_id.get_untracked().as_deref(),
                        &commit_for_click,
                    ) {
                        HistorySelectionAction::ToggleClosed => {
                            set_notice.set(None);
                            set_commit_diff_request_id.set(None);
                            set_commit_diff_result.set(Vec::new());
                            selected_commit.set(None);
                        }
                        HistorySelectionAction::ClearBaseSelection => {
                            set_notice.set(None);
                            set_commit_diff_request_id.set(None);
                            set_commit_diff_result.set(Vec::new());
                            selected_commit.set(None);
                            compare_base_commit_id.set(None);
                        }
                        HistorySelectionAction::ShowParentDiff {
                            parent_id,
                            target_id,
                        } => {
                            set_notice.set(None);
                            set_commit_diff_result.set(Vec::new());
                            on_get_commit_diff.run((parent_id, target_id.clone()));
                            selected_commit.set(Some(target_id));
                        }
                        HistorySelectionAction::ShowRangeDiff { base_id, target_id } => {
                            set_notice.set(None);
                            set_commit_diff_result.set(Vec::new());
                            on_get_commit_diff.run((Some(base_id), target_id.clone()));
                            selected_commit.set(Some(target_id));
                        }
                    }
                }
            >
                <div class="text-[13px] text-primary leading-tight mb-0.5 font-medium truncate" title={commit.message.clone()}>
                    {commit.message.clone()}
                </div>
                <div class="flex items-center gap-2 text-[11px] text-muted">
                    <span class="font-mono bg-hover px-1 rounded text-secondary">{short_commit_id(&commit.id)}</span>
                    <span>{move || format_relative(commit.timestamp, locale.get())}</span>
                    <Show when=move || visual_state.get() == HistoryCommitVisualState::Base>
                        <span class="ml-auto rounded px-1.5 py-0.5 text-[10px] font-medium bg-accent/15 text-accent">
                            {move || source_control::history_base_badge(locale.get())}
                        </span>
                    </Show>
                    <Show when=move || visual_state.get() == HistoryCommitVisualState::CompareTarget>
                        <span class="ml-auto rounded px-1.5 py-0.5 text-[10px] font-medium bg-hover text-secondary ring-1 ring-accent/20">
                            {move || source_control::history_target_badge(locale.get())}
                        </span>
                    </Show>
                </div>
            </div>
            <Show when=move || selected_commit.get().as_deref() == Some(selected_commit_id.as_str())>
                <div class="ml-2 mt-1 border-l border-active pl-2">
                    <HistoryCommitDetails
                        locale
                        compare_base_commit_id
                        base_commit_id=compare_base_commit_id.get().or_else(|| parent_commit_id.clone())
                        target_commit_id=commit.id.clone()
                        commit_diff_request_id
                        commit_diff_result
                        notice
                    />
                </div>
            </Show>
        </div>
    }
}
