use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

use super::super::diff_session::DiffSessionWire;
use super::super::effects_sc_scope::matches_current_scope;
use super::super::effects_sc_state::scoped_ack_matches;
use super::super::types::PendingBranchTarget;

pub(crate) struct ScMessageContext<'a> {
    pub(crate) set_staged: WriteSignal<Vec<ChangeEntry>>,
    pub(crate) set_unstaged: WriteSignal<Vec<ChangeEntry>>,
    pub(crate) changes_request_id: ReadSignal<Option<String>>,
    pub(crate) set_changes_request_id: WriteSignal<Option<String>>,
    pub(crate) set_history: WriteSignal<Vec<CommitInfo>>,
    pub(crate) commit_history_request_id: ReadSignal<Option<String>>,
    pub(crate) set_commit_history_request_id: WriteSignal<Option<String>>,
    pub(crate) set_doc_list_request_id: WriteSignal<Option<String>>,
    pub(crate) set_tree_request_id: WriteSignal<Option<String>>,
    pub(crate) doc_diff_request_id: ReadSignal<Option<String>>,
    pub(crate) set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub(crate) set_diff: WriteSignal<Option<DiffSessionWire>>,
    pub(crate) commit_diff_request_id: ReadSignal<Option<String>>,
    pub(crate) set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub(crate) set_commit_diff: WriteSignal<Vec<CommitFileDiff>>,
    pub(crate) current_repo_id: ReadSignal<Option<String>>,
    pub(crate) active_branch: ReadSignal<Option<PeerId>>,
    pub(crate) pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub(crate) pending_repo_switch: ReadSignal<Option<String>>,
    pub(crate) current_scope_nonce: ReadSignal<u64>,
    pub(crate) schedule_refresh: &'a dyn Fn(),
    pub(crate) ws: &'a WsService,
}

impl ScMessageContext<'_> {
    pub(crate) fn active_scope_nonce(&self) -> u64 {
        self.current_scope_nonce.get_untracked()
    }

    pub(crate) fn in_scope(&self, repo_id: &Option<uuid::Uuid>, branch: &Option<PeerId>) -> bool {
        matches_current_scope(
            repo_id,
            branch,
            self.current_repo_id,
            self.active_branch,
            self.pending_branch_switch,
            self.pending_repo_switch,
        )
    }

    pub(crate) fn in_ack_scope(
        &self,
        repo_id: &Option<uuid::Uuid>,
        branch: &Option<PeerId>,
        scope_nonce: Option<u64>,
    ) -> bool {
        self.in_scope(repo_id, branch) && scoped_ack_matches(scope_nonce, self.active_scope_nonce())
    }

    pub(crate) fn schedule_refresh(&self) {
        (self.schedule_refresh)();
    }
}
