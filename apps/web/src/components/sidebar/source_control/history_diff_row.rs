//! Lazy commit-file projection request row.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 10_rendering#document-authority-bridge

use crate::components::icons::FileText;
use crate::hooks::use_core::SourceControlContext;
use crate::runtime::source_control_client::diff_cache::{
    commit_projection_cache_key, get_projection, projection_scope_key,
};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use crate::runtime::{scope_client::ScopeClient, session_client::SessionClient};
use deve_core::protocol::ClientMessage;
use deve_core::source_control::{ChangeStatus, CommitFileDiffSummary};
use leptos::prelude::*;

#[component]
pub fn HistoryDiffRow(
    file: CommitFileDiffSummary,
    commit_a: Option<String>,
    commit_b: String,
) -> impl IntoView {
    let source_control = expect_context::<SourceControlContext>();
    let session = expect_context::<SessionClient>();
    let scope = expect_context::<ScopeClient>();
    let path_label = file
        .previous_path
        .as_ref()
        .map(|old| format!("{old} -> {}", file.path))
        .unwrap_or_else(|| file.path.clone());
    let canonical_path = file.path.clone();
    let display_path = path_label.clone();
    let target = file.target.clone();
    let (marker, class_name) = match file.status {
        ChangeStatus::Modified => ("M", "text-modified"),
        ChangeStatus::Added => ("A", "text-added"),
        ChangeStatus::Deleted => ("D", "text-deleted"),
        ChangeStatus::Renamed => ("R", "text-added"),
    };

    view! {
        <button
            type="button"
            class="w-full flex items-center gap-1 text-[12px] text-secondary py-0.5 hover:bg-hover px-1 rounded cursor-pointer text-left"
            on:click=move |_| {
                source_control.clear_notice.run(());
                let cache_key = commit_projection_cache_key(
                    commit_a.as_deref(),
                    &commit_b,
                    &target,
                );
                let scope_key = projection_scope_key(
                    scope.current_repo_id.get_untracked().as_deref(),
                    scope.active_branch.get_untracked().as_ref(),
                    scope.current_scope_nonce.get_untracked(),
                );
                if let Some(projection) = get_projection(&scope_key, &cache_key) {
                    source_control.set_diff_content.set(Some(
                        DiffSessionWire::with_projection_and_display_path(
                            canonical_path.clone(),
                            display_path.clone(),
                            projection,
                        )
                        .with_doc_id(Some(target.doc_id))
                        .with_cache_key(cache_key),
                    ));
                    return;
                }
                let request_id = uuid::Uuid::new_v4().to_string();
                source_control.set_commit_diff_request_id.set(Some(request_id.clone()));
                source_control.set_diff_content.set(Some(
                    DiffSessionWire::loading(canonical_path.clone(), display_path.clone())
                        .with_doc_id(Some(target.doc_id))
                        .with_pending_request(request_id.clone())
                        .with_cache_key(cache_key),
                ));
                session.ws.send(ClientMessage::GetCommitFileDiff {
                    request_id,
                    commit_a: commit_a.clone(),
                    commit_b: commit_b.clone(),
                    target: target.clone(),
                    scope_nonce: Some(scope.current_scope_nonce.get_untracked()),
                });
            }
        >
            <FileText class="w-3 h-3 text-muted" />
            <span class="truncate flex-1">{path_label}</span>
            <span class=format!("{class_name} text-[10px] font-bold")>{marker}</span>
        </button>
    }
}
