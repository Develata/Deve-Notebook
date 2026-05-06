use super::super::effects_sc_scope::{matches_current_repo, matches_current_scope};
use super::super::effects_sc_state::{
    changes_list_matches_request, clear_repo_scoped_state, commit_diff_matches_request,
    commit_history_matches_request, doc_diff_matches_request, scoped_ack_matches,
};
use crate::hooks::use_core::{PendingBranchTarget, diff_session::DiffSessionWire};
use deve_core::models::PeerId;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

#[path = "effects_sc_test_ack.rs"]
mod ack;
#[path = "effects_sc_test_commit_diff.rs"]
mod commit_diff;
#[path = "effects_sc_test_doc_diff.rs"]
mod doc_diff;
#[path = "effects_sc_test_read_lists.rs"]
mod read_lists;
#[path = "effects_sc_test_request.rs"]
mod request;
#[path = "effects_sc_test_reset.rs"]
mod reset;
#[path = "effects_sc_test_scope.rs"]
mod scope;
