use crate::components::sidebar::source_control::history_compare_logic::{
    HistorySelectionAction, resolve_history_selection, short_commit_id,
};
use crate::components::sidebar::source_control::history_diff_row::HistoryDiffRow;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::i18n::Locale;
use crate::utils::time::format_relative;
use deve_core::source_control::{CommitFileDiff, CommitInfo};
use leptos::prelude::*;

#[component]
pub fn HistoryCommitItem(
    locale: RwSignal<Locale>,
    read_blocked: Signal<bool>,
    selected_commit: RwSignal<Option<String>>,
    compare_base_commit_id: RwSignal<Option<String>>,
    commit: CommitInfo,
    commit_diff_request_id: ReadSignal<Option<String>>,
    commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    notice: ReadSignal<Option<SourceControlNotice>>,
    set_commit_diff_result: WriteSignal<Vec<CommitFileDiff>>,
    on_get_commit_diff: Callback<(Option<String>, String)>,
) -> impl IntoView {
    let commit_id = commit.id.clone();
    let selected_commit_id = commit_id.clone();
    let commit_for_click = commit.clone();

    view! {
        <div class="relative mb-3 group">
            <div class="absolute -left-[19px] top-[3px] w-2.5 h-2.5 rounded-full border-2 border-white bg-accent shadow-sm z-10"></div>
            <div
                class=move || {
                    if read_blocked.get() {
                        "pr-2 cursor-default".to_string()
                    } else {
                        "pr-2 cursor-pointer".to_string()
                    }
                }
                on:click=move |_| {
                    if read_blocked.get_untracked() {
                        return;
                    }
                    match resolve_history_selection(
                        selected_commit.get_untracked().as_deref(),
                        compare_base_commit_id.get_untracked().as_deref(),
                        &commit_for_click,
                    ) {
                        HistorySelectionAction::ToggleClosed => {
                            set_commit_diff_result.set(Vec::new());
                            selected_commit.set(None);
                        }
                        HistorySelectionAction::ShowParentDiff {
                            parent_id,
                            target_id,
                        } => {
                            set_commit_diff_result.set(Vec::new());
                            on_get_commit_diff.run((parent_id, target_id.clone()));
                            selected_commit.set(Some(target_id));
                        }
                        HistorySelectionAction::ShowRangeDiff { base_id, target_id } => {
                            set_commit_diff_result.set(Vec::new());
                            on_get_commit_diff.run((Some(base_id), target_id.clone()));
                            selected_commit.set(Some(target_id));
                        }
                        HistorySelectionAction::IgnoreBaseCommit => {}
                    }
                }
            >
                <div class="text-[13px] text-primary leading-tight mb-0.5 font-medium truncate" title={commit.message.clone()}>
                    {commit.message.clone()}
                </div>
                <div class="flex items-center gap-2 text-[11px] text-muted">
                    <span class="font-mono bg-hover px-1 rounded text-secondary">{short_commit_id(&commit.id)}</span>
                    <span>{format_relative(commit.timestamp)}</span>
                    <Show when=move || compare_base_commit_id.get().as_deref() == Some(commit_id.as_str())>
                        <span class="ml-auto rounded px-1.5 py-0.5 text-[10px] font-medium bg-accent/15 text-accent">
                            {move || match locale.get() {
                                Locale::En => "Base",
                                Locale::Zh => "基准",
                            }}
                        </span>
                    </Show>
                </div>
            </div>
            <Show when=move || selected_commit.get().as_deref() == Some(selected_commit_id.as_str())>
                <div class="ml-2 mt-1 border-l border-active pl-2">
                    {move || {
                        if commit_diff_request_id.get().is_some() {
                            view! {
                                <div class="py-1 text-[12px] text-muted">
                                    {match locale.get() {
                                        Locale::En => "Loading commit diff...",
                                        Locale::Zh => "正在加载提交差异...",
                                    }}
                                </div>
                            }.into_any()
                        } else if !commit_diff_result.get().is_empty() {
                            view! {
                                {commit_diff_result.get().into_iter().map(|file| view! { <HistoryDiffRow file /> }).collect_view()}
                            }.into_any()
                        } else if notice.get().is_none() {
                            view! {
                                <div class="py-1 text-[12px] text-muted">
                                    {match locale.get() {
                                        Locale::En => "No file-level diff available for this commit.",
                                        Locale::Zh => "这个提交没有可展示的文件级差异。",
                                    }}
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}
