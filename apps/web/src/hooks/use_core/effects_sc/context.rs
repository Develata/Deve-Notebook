//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

use super::super::effects_sc_scope::matches_current_scope;
use super::super::effects_sc_state::scoped_ack_matches;
use super::super::source_control_notice::SourceControlNotice;
use super::super::state::CoreSignals;
use super::super::types::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;

pub(crate) struct ScMessageContext<'a> {
    pub(crate) set_staged: WriteSignal<Vec<ChangeEntry>>,
    pub(crate) set_unstaged: WriteSignal<Vec<ChangeEntry>>,
    pub(crate) set_confirmed: WriteSignal<Vec<ChangeEntry>>,
    pub(crate) changes_request_id: ReadSignal<Option<String>>,
    pub(crate) set_changes_request_id: WriteSignal<Option<String>>,
    pub(crate) set_history: WriteSignal<Vec<CommitInfo>>,
    pub(crate) commit_history_request_id: ReadSignal<Option<String>>,
    pub(crate) set_commit_history_request_id: WriteSignal<Option<String>>,
    pub(crate) set_doc_list_request_id: WriteSignal<Option<String>>,
    pub(crate) set_tree_request_id: WriteSignal<Option<String>>,
    pub(crate) degraded_sync_mode: ReadSignal<Option<crate::storage::DegradedSyncMode>>,
    pub(crate) sync_banner: ReadSignal<Option<String>>,
    pub(crate) set_sync_banner: WriteSignal<Option<String>>,
    pub(crate) doc_diff_request_id: ReadSignal<Option<String>>,
    pub(crate) set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub(crate) diff: ReadSignal<Option<DiffSessionWire>>,
    pub(crate) set_diff: WriteSignal<Option<DiffSessionWire>>,
    pub(crate) commit_diff_request_id: ReadSignal<Option<String>>,
    pub(crate) set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub(crate) set_commit_diff: WriteSignal<Vec<CommitFileDiff>>,
    pub(crate) set_notice: WriteSignal<Option<SourceControlNotice>>,
    pub(crate) current_repo_id: ReadSignal<Option<String>>,
    pub(crate) load_state: ReadSignal<LoadPhase>,
    pub(crate) is_spectator: Signal<bool>,
    pub(crate) handshake_ready: ReadSignal<bool>,
    pub(crate) active_branch: ReadSignal<Option<PeerId>>,
    pub(crate) pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub(crate) pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub(crate) current_scope_nonce: ReadSignal<u64>,
    pub(crate) schedule_refresh: &'a dyn Fn(),
    pub(crate) ws: &'a WsService,
}

impl ScMessageContext<'_> {
    pub(crate) fn from_core_signals<'a>(
        signals: CoreSignals,
        schedule_refresh: &'a dyn Fn(),
        ws: &'a WsService,
    ) -> ScMessageContext<'a> {
        ScMessageContext {
            set_staged: signals.set_staged_changes,
            set_unstaged: signals.set_unstaged_changes,
            set_confirmed: signals.set_confirmed_changes,
            changes_request_id: signals.changes_request_id,
            set_changes_request_id: signals.set_changes_request_id,
            set_history: signals.set_commit_history,
            commit_history_request_id: signals.commit_history_request_id,
            set_commit_history_request_id: signals.set_commit_history_request_id,
            set_doc_list_request_id: signals.set_doc_list_request_id,
            set_tree_request_id: signals.set_tree_request_id,
            degraded_sync_mode: signals.degraded_sync_mode,
            sync_banner: signals.sync_banner,
            set_sync_banner: signals.set_sync_banner,
            doc_diff_request_id: signals.doc_diff_request_id,
            set_doc_diff_request_id: signals.set_doc_diff_request_id,
            diff: signals.diff_content,
            set_diff: signals.set_diff_content,
            commit_diff_request_id: signals.commit_diff_request_id,
            set_commit_diff_request_id: signals.set_commit_diff_request_id,
            set_commit_diff: signals.set_commit_diff_result,
            set_notice: signals.set_source_control_notice,
            current_repo_id: signals.current_repo_id,
            load_state: signals.load_state,
            is_spectator: signals.is_spectator.into(),
            handshake_ready: signals.handshake_ready,
            active_branch: signals.active_branch,
            pending_branch_switch: signals.pending_branch_switch,
            pending_repo_switch: signals.pending_repo_switch,
            current_scope_nonce: signals.current_scope_nonce,
            schedule_refresh,
            ws,
        }
    }

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
