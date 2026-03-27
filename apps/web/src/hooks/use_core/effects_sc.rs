//! Source Control 消息分发。

#[path = "effects_sc_context.rs"]
mod context;
#[path = "effects_sc_dispatch.rs"]
mod dispatch;
#[path = "effects_sc_dispatch_acks.rs"]
mod dispatch_acks;
#[path = "effects_sc_dispatch_lists.rs"]
mod dispatch_lists;

use crate::api::WsService;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

use super::diff_session::DiffSessionWire;
use super::types::PendingBranchTarget;
pub(crate) use context::ScMessageContext;
pub(crate) use dispatch::handle_sc_message;

#[allow(unused_imports)]
pub(crate) use super::effects_sc_scope::{matches_current_repo, matches_current_scope};
pub(crate) use super::effects_sc_state::clear_repo_scoped_state;
#[cfg(test)]
pub(crate) use super::effects_sc_state::{
    changes_list_matches_request, commit_diff_matches_request, commit_history_matches_request,
    doc_diff_matches_request, scoped_ack_matches,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn sc_message_context<'a>(
    set_staged: WriteSignal<Vec<ChangeEntry>>,
    set_unstaged: WriteSignal<Vec<ChangeEntry>>,
    changes_request_id: ReadSignal<Option<String>>,
    set_changes_request_id: WriteSignal<Option<String>>,
    set_history: WriteSignal<Vec<CommitInfo>>,
    commit_history_request_id: ReadSignal<Option<String>>,
    set_commit_history_request_id: WriteSignal<Option<String>>,
    set_doc_list_request_id: WriteSignal<Option<String>>,
    set_tree_request_id: WriteSignal<Option<String>>,
    doc_diff_request_id: ReadSignal<Option<String>>,
    set_doc_diff_request_id: WriteSignal<Option<String>>,
    set_diff: WriteSignal<Option<DiffSessionWire>>,
    commit_diff_request_id: ReadSignal<Option<String>>,
    set_commit_diff_request_id: WriteSignal<Option<String>>,
    set_commit_diff: WriteSignal<Vec<CommitFileDiff>>,
    current_repo_id: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<deve_core::models::PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pending_repo_switch: ReadSignal<Option<String>>,
    current_scope_nonce: ReadSignal<u64>,
    schedule_refresh: &'a dyn Fn(),
    ws: &'a WsService,
) -> ScMessageContext<'a> {
    ScMessageContext {
        set_staged,
        set_unstaged,
        changes_request_id,
        set_changes_request_id,
        set_history,
        commit_history_request_id,
        set_commit_history_request_id,
        set_doc_list_request_id,
        set_tree_request_id,
        doc_diff_request_id,
        set_doc_diff_request_id,
        set_diff,
        commit_diff_request_id,
        set_commit_diff_request_id,
        set_commit_diff,
        current_repo_id,
        active_branch,
        pending_branch_switch,
        pending_repo_switch,
        current_scope_nonce,
        schedule_refresh,
        ws,
    }
}

#[cfg(test)]
#[path = "effects_sc_test.rs"]
mod tests;
