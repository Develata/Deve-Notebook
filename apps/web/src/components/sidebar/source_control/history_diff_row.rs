//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 10_rendering#document-authority-bridge
//!
use crate::components::icons::FileText;
use crate::hooks::use_core::SourceControlContext;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::source_control::{ChangeStatus, CommitFileDiff};
use leptos::prelude::*;

#[component]
pub fn HistoryDiffRow(file: CommitFileDiff) -> impl IntoView {
    let source_control = expect_context::<SourceControlContext>();
    let path_label = file
        .previous_path
        .as_ref()
        .map(|old| format!("{old} -> {}", file.path))
        .unwrap_or_else(|| file.path.clone());
    let canonical_path = file.path.clone();
    let diff_display_path = path_label.clone();
    let (marker, class_name) = match file.status {
        ChangeStatus::Modified => ("M", "text-modified"),
        ChangeStatus::Added => ("A", "text-added"),
        ChangeStatus::Deleted => ("D", "text-deleted"),
        ChangeStatus::Renamed => ("R", "text-added"),
    };

    view! {
        <button
            class="w-full flex items-center gap-1 text-[12px] text-secondary py-0.5 hover:bg-hover px-1 rounded cursor-pointer text-left"
            on:click=move |_| {
                source_control.clear_notice.run(());
                source_control
                    .set_diff_content
                    .set(Some(DiffSessionWire::with_display_path(
                        canonical_path.clone(),
                        diff_display_path.clone(),
                        file.old_content.clone(),
                        file.new_content.clone(),
                    )));
            }
        >
            <FileText class="w-3 h-3 text-muted" />
            <span class="truncate flex-1">{path_label}</span>
            <span class=format!("{class_name} text-[10px] font-bold")>{marker}</span>
        </button>
    }
}
