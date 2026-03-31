use crate::components::icons::FileText;
use deve_core::source_control::{ChangeStatus, CommitFileDiff};
use leptos::prelude::*;

#[component]
pub fn HistoryDiffRow(file: CommitFileDiff) -> impl IntoView {
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
