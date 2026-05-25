//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
use super::super::callbacks::{MiscRequestSignals, SearchScopeSignals};
use super::super::callbacks_sc::{SourceControlRequestSignals, SourceControlScopeSignals};
use super::super::callbacks_scope::LocalScopeSignals;
use super::super::state::CoreSignals;
use super::super::types::SwitchScopeSignals;
use super::super::write_gate::RepoWriteSignals;

pub(super) fn local_scope(signals: &CoreSignals) -> LocalScopeSignals {
    LocalScopeSignals {
        current_repo_id: signals.current_repo_id,
        current_scope_nonce: signals.current_scope_nonce,
        active_branch: signals.active_branch,
        pending_branch_switch: signals.pending_branch_switch,
        pending_repo_switch: signals.pending_repo_switch,
    }
}

pub(super) fn repo_write(signals: &CoreSignals) -> RepoWriteSignals {
    RepoWriteSignals {
        load_state: signals.load_state,
        is_spectator: signals.is_spectator.into(),
        handshake_ready: signals.handshake_ready,
        current_repo_id: signals.current_repo_id,
        current_scope_nonce: signals.current_scope_nonce,
        active_branch: signals.active_branch,
        pending_branch_switch: signals.pending_branch_switch,
        pending_repo_switch: signals.pending_repo_switch,
    }
}

pub(super) fn source_control_scope(signals: &CoreSignals) -> SourceControlScopeSignals {
    SourceControlScopeSignals {
        current_repo_id: signals.current_repo_id,
        active_branch: signals.active_branch,
        current_scope_nonce: signals.current_scope_nonce,
        pending_branch_switch: signals.pending_branch_switch,
        pending_repo_switch: signals.pending_repo_switch,
    }
}

pub(super) fn source_control_requests(signals: &CoreSignals) -> SourceControlRequestSignals {
    SourceControlRequestSignals {
        set_changes_request_id: signals.set_changes_request_id,
        set_commit_history_request_id: signals.set_commit_history_request_id,
        set_doc_diff_request_id: signals.set_doc_diff_request_id,
        set_commit_diff_request_id: signals.set_commit_diff_request_id,
    }
}

pub(super) fn search_scope(signals: &CoreSignals) -> SearchScopeSignals {
    SearchScopeSignals {
        current_scope_nonce: signals.current_scope_nonce,
        pending_branch_switch: signals.pending_branch_switch,
        pending_repo_switch: signals.pending_repo_switch,
    }
}

pub(super) fn misc_requests(signals: &CoreSignals) -> MiscRequestSignals {
    MiscRequestSignals {
        set_plugin_request_ids: signals.set_plugin_request_ids,
        set_search_request_id: signals.set_search_request_id,
        set_search_results: signals.set_search_results,
    }
}

pub(super) fn switch_scope(signals: &CoreSignals) -> SwitchScopeSignals {
    SwitchScopeSignals {
        current_doc: signals.current_doc,
        pending_local_edits: signals.pending_local_edits,
        set_pending_navigation: signals.set_pending_navigation,
        current_repo: signals.current_repo,
        current_repo_id: signals.current_repo_id,
        current_scope_nonce: signals.current_scope_nonce,
        active_branch: signals.active_branch,
        pending_branch_switch: signals.pending_branch_switch,
        pending_branch_switch_nonce: signals.pending_branch_switch_nonce,
        pending_repo_switch: signals.pending_repo_switch,
        pending_repo_switch_nonce: signals.pending_repo_switch_nonce,
        set_handshake_ready: signals.set_handshake_ready,
        set_handshake_scope_nonce: signals.set_handshake_scope_nonce,
        set_pending_branch_switch: signals.set_pending_branch_switch,
        set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
        set_pending_repo_switch: signals.set_pending_repo_switch,
        set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
    }
}
