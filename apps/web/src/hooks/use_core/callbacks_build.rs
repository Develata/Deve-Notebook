use crate::api::WsService;

use super::SwitchScopeSignals;
use super::callbacks::{
    self, create_doc_callbacks, create_misc_callbacks, create_source_control_callbacks,
    create_switch_callbacks, create_sync_callbacks,
};
use super::callbacks_sc::{SourceControlRequestSignals, SourceControlScopeSignals};
use super::callbacks_scope::LocalScopeSignals;
use super::state::CoreSignals;
use super::state_callbacks::CoreStateCallbacks;
use super::write_gate::RepoWriteSignals;

fn local_scope(signals: &CoreSignals) -> LocalScopeSignals {
    LocalScopeSignals {
        current_repo_id: signals.current_repo_id,
        current_scope_nonce: signals.current_scope_nonce,
        active_branch: signals.active_branch,
        pending_branch_switch: signals.pending_branch_switch,
        pending_repo_switch: signals.pending_repo_switch,
    }
}

fn repo_write(signals: &CoreSignals) -> RepoWriteSignals {
    RepoWriteSignals {
        load_state: signals.load_state,
        is_spectator: signals.is_spectator.into(),
        handshake_ready: signals.handshake_ready,
        current_repo_id: signals.current_repo_id,
        pending_branch_switch: signals.pending_branch_switch,
        pending_repo_switch: signals.pending_repo_switch,
    }
}

pub(super) fn build_callbacks(ws: &WsService, signals: &CoreSignals) -> CoreStateCallbacks {
    let doc = create_doc_callbacks(
        ws,
        signals.current_doc,
        local_scope(signals),
        repo_write(signals),
        signals.pending_local_edits,
        signals.set_pending_navigation,
        signals.set_current_doc,
        signals.set_pending_created_doc_path,
        signals.set_explicit_home,
    );
    let sync = create_sync_callbacks(
        ws,
        signals.current_doc,
        local_scope(signals),
        repo_write(signals),
        signals.set_shadow_list_request_id,
        signals.set_sync_mode_request_id,
        signals.set_pending_ops_request_id,
    );
    let sc = create_source_control_callbacks(
        ws,
        SourceControlScopeSignals {
            current_repo_id: signals.current_repo_id,
            active_branch: signals.active_branch,
            current_scope_nonce: signals.current_scope_nonce,
            pending_branch_switch: signals.pending_branch_switch,
            pending_repo_switch: signals.pending_repo_switch,
        },
        repo_write(signals),
        SourceControlRequestSignals {
            set_changes_request_id: signals.set_changes_request_id,
            set_commit_history_request_id: signals.set_commit_history_request_id,
            set_doc_diff_request_id: signals.set_doc_diff_request_id,
            set_commit_diff_request_id: signals.set_commit_diff_request_id,
        },
    );
    let misc = create_misc_callbacks(
        ws,
        signals.set_stats,
        signals.load_state,
        callbacks::SearchScopeSignals {
            current_scope_nonce: signals.current_scope_nonce,
            pending_branch_switch: signals.pending_branch_switch,
            pending_repo_switch: signals.pending_repo_switch,
        },
        callbacks::MiscRequestSignals {
            set_plugin_request_ids: signals.set_plugin_request_ids,
            set_search_request_id: signals.set_search_request_id,
        },
    );
    let switch = create_switch_callbacks(
        ws,
        SwitchScopeSignals {
            current_doc: signals.current_doc,
            pending_local_edits: signals.pending_local_edits,
            set_pending_navigation: signals.set_pending_navigation,
            current_repo: signals.current_repo,
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
        },
    );
    CoreStateCallbacks::new(doc, sync, sc, misc, switch)
}
