use crate::components::icons::FileText;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::i18n::Locale;
use crate::utils::time::format_relative;
use deve_core::source_control::{ChangeStatus, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

#[component]
pub fn HistoryTimeline(
    locale: RwSignal<Locale>,
    read_blocked: Signal<bool>,
    selected_commit: RwSignal<Option<String>>,
    commit_history: ReadSignal<Vec<CommitInfo>>,
    commit_diff_request_id: ReadSignal<Option<String>>,
    commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    notice: ReadSignal<Option<SourceControlNotice>>,
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
                    let commit_id = commit.id.clone();
                    let parent_id = commit.parent_id.clone();
                    let selected_commit_id = commit_id.clone();
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
                                    if selected_commit.get_untracked().as_deref()
                                        == Some(commit_id.as_str())
                                    {
                                        set_commit_diff_result.set(Vec::new());
                                        selected_commit.set(None);
                                    } else {
                                        set_commit_diff_result.set(Vec::new());
                                        on_get_commit_diff
                                            .run((parent_id.clone(), commit_id.clone()));
                                        selected_commit.set(Some(commit_id.clone()));
                                    }
                                }
                            >
                                <div class="text-[13px] text-primary leading-tight mb-0.5 font-medium truncate" title={commit.message.clone()}>
                                    {commit.message.clone()}
                                </div>
                                <div class="flex items-center gap-2 text-[11px] text-muted">
                                    <span class="font-mono bg-hover px-1 rounded text-secondary">{commit.id[0..7].to_string()}</span>
                                    <span>{format_relative(commit.timestamp)}</span>
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
                                                {commit_diff_result.get().into_iter().map(render_diff_row).collect_view()}
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
            />
        </div>
    }
}

fn render_diff_row(file: CommitFileDiff) -> impl IntoView {
    let path_label = file
        .previous_path
        .as_ref()
        .map(|old| format!("{old} -> {}", file.path))
        .unwrap_or_else(|| file.path.clone());
    let (marker, class_name) = match file.status {
        ChangeStatus::Modified => ("M", "text-modified"),
        ChangeStatus::Added => ("A", "text-added"),
        ChangeStatus::Deleted => ("D", "text-deleted"),
        ChangeStatus::Renamed => ("R", "text-added"),
    };

    view! {
        <div class="flex items-center gap-1 text-[12px] text-secondary py-0.5 hover:bg-hover px-1 rounded cursor-pointer">
            <FileText class="w-3 h-3 text-muted" />
            <span class="truncate flex-1">{path_label}</span>
            <span class=format!("{class_name} text-[10px] font-bold")>{marker}</span>
        </div>
    }
}
