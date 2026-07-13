use super::super::effects_sc_scope::{matches_current_repo, matches_current_scope};
use super::super::effects_sc_state::{
    changes_list_matches_request, clear_repo_scoped_state, commit_diff_matches_request,
    commit_history_matches_request, doc_diff_matches_request, scoped_ack_matches,
};
use crate::hooks::use_core::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use deve_core::models::{DocId, PeerId};
use deve_core::source_control::{
    ChangeEntry, ChangeStatus, CommitFileDiffSummary, CommitFileDiffTarget, CommitInfo,
};
use leptos::prelude::*;
use std::sync::Arc;

fn test_projection(
    old: &str,
    new: &str,
) -> Arc<deve_core::source_control::diff_projection::DiffProjection> {
    Arc::new(
        deve_core::source_control::diff_projection::compute_diff_projection(
            old.to_string(),
            new.to_string(),
        )
        .expect("test projection"),
    )
}

fn test_commit_summary(
    path: &str,
    status: ChangeStatus,
    previous_path: Option<&str>,
) -> CommitFileDiffSummary {
    let doc_id = DocId::new();
    let previous_path = previous_path.map(str::to_string);
    CommitFileDiffSummary {
        doc_id,
        path: path.to_string(),
        status,
        previous_path: previous_path.clone(),
        target: CommitFileDiffTarget {
            doc_id,
            path: path.to_string(),
            previous_path,
            status,
        },
    }
}

mod ack;
mod commit_diff;
mod doc_diff;
mod read_lists;
mod request;
mod reset;
mod scope;
