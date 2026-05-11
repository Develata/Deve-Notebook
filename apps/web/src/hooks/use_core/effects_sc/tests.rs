use super::super::effects_sc_scope::{matches_current_repo, matches_current_scope};
use super::super::effects_sc_state::{
    changes_list_matches_request, clear_repo_scoped_state, commit_diff_matches_request,
    commit_history_matches_request, doc_diff_matches_request, scoped_ack_matches,
};
use crate::hooks::use_core::{PendingBranchTarget, diff_session::DiffSessionWire};
use deve_core::models::PeerId;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

mod ack;
mod commit_diff;
mod doc_diff;
mod read_lists;
mod request;
mod reset;
mod scope;
