//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
use crate::storage::DegradedSyncMode;
use deve_core::models::PeerId;
use deve_core::protocol::RepoListEntry;
use deve_core::tree::FileNode;
use leptos::prelude::*;

use super::super::types::{PendingBranchSwitch, PendingRepoSwitch};
mod projection;
mod scope;
use self::projection::init_repo_projection_signals;
use self::scope::init_repo_scope_signals;

#[derive(Clone, Copy)]
pub(super) struct RepoSignals {
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchSwitch>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub set_current_repo_id: WriteSignal<Option<String>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub set_pending_repo_switch: WriteSignal<Option<PendingRepoSwitch>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub set_current_scope_nonce: WriteSignal<u64>,
    pub remove_scope_partial_stage:
        ReadSignal<Option<crate::runtime::remove_scope_partial::RemoveScopePartialStage>>,
    pub set_remove_scope_partial_stage:
        WriteSignal<Option<crate::runtime::remove_scope_partial::RemoveScopePartialStage>>,
    pub explicit_repo_selection_required: ReadSignal<bool>,
    pub set_explicit_repo_selection_required: WriteSignal<bool>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub set_shadow_repos: WriteSignal<Vec<String>>,
    pub shadow_list_request_id: ReadSignal<Option<String>>,
    pub set_shadow_list_request_id: WriteSignal<Option<String>>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub set_repo_list: WriteSignal<Vec<String>>,
    pub repo_entries: ReadSignal<Vec<RepoListEntry>>,
    pub set_repo_entries: WriteSignal<Vec<RepoListEntry>>,
    pub repo_list_request_id: ReadSignal<Option<String>>,
    pub set_repo_list_request_id: WriteSignal<Option<String>>,
    pub doc_list_request_id: ReadSignal<Option<String>>,
    pub set_doc_list_request_id: WriteSignal<Option<String>>,
    pub tree_request_id: ReadSignal<Option<String>>,
    pub set_tree_request_id: WriteSignal<Option<String>>,
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
    pub set_tree_nodes: WriteSignal<Vec<FileNode>>,
    pub degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    pub set_degraded_sync_mode: WriteSignal<Option<DegradedSyncMode>>,
    pub sync_banner: ReadSignal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
}

pub(super) fn init_repo_signals() -> RepoSignals {
    let scope = init_repo_scope_signals();
    let projection = init_repo_projection_signals();

    RepoSignals {
        active_branch: scope.active_branch,
        set_active_branch: scope.set_active_branch,
        pending_branch_switch: scope.pending_branch_switch,
        set_pending_branch_switch: scope.set_pending_branch_switch,
        current_repo: scope.current_repo,
        set_current_repo: scope.set_current_repo,
        current_repo_id: scope.current_repo_id,
        set_current_repo_id: scope.set_current_repo_id,
        pending_repo_switch: scope.pending_repo_switch,
        set_pending_repo_switch: scope.set_pending_repo_switch,
        current_scope_nonce: scope.current_scope_nonce,
        set_current_scope_nonce: scope.set_current_scope_nonce,
        remove_scope_partial_stage: scope.remove_scope_partial_stage,
        set_remove_scope_partial_stage: scope.set_remove_scope_partial_stage,
        explicit_repo_selection_required: scope.explicit_repo_selection_required,
        set_explicit_repo_selection_required: scope.set_explicit_repo_selection_required,
        shadow_repos: projection.shadow_repos,
        set_shadow_repos: projection.set_shadow_repos,
        shadow_list_request_id: projection.shadow_list_request_id,
        set_shadow_list_request_id: projection.set_shadow_list_request_id,
        repo_list: projection.repo_list,
        set_repo_list: projection.set_repo_list,
        repo_entries: projection.repo_entries,
        set_repo_entries: projection.set_repo_entries,
        repo_list_request_id: projection.repo_list_request_id,
        set_repo_list_request_id: projection.set_repo_list_request_id,
        doc_list_request_id: projection.doc_list_request_id,
        set_doc_list_request_id: projection.set_doc_list_request_id,
        tree_request_id: projection.tree_request_id,
        set_tree_request_id: projection.set_tree_request_id,
        tree_nodes: projection.tree_nodes,
        set_tree_nodes: projection.set_tree_nodes,
        degraded_sync_mode: scope.degraded_sync_mode,
        set_degraded_sync_mode: scope.set_degraded_sync_mode,
        sync_banner: scope.sync_banner,
        set_sync_banner: scope.set_sync_banner,
    }
}
