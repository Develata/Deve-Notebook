//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
use crate::storage::DegradedSyncMode;
use deve_core::models::PeerId;
use deve_core::tree::FileNode;
use leptos::prelude::*;

use super::super::types::PendingBranchTarget;
#[path = "repo_projection.rs"]
mod repo_projection;
#[path = "repo_scope.rs"]
mod repo_scope;
use self::repo_projection::init_repo_projection_signals;
use self::repo_scope::init_repo_scope_signals;

#[derive(Clone, Copy)]
pub(super) struct RepoSignals {
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    pub pending_branch_switch_nonce: ReadSignal<Option<u64>>,
    pub set_pending_branch_switch_nonce: WriteSignal<Option<u64>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub set_current_repo_id: WriteSignal<Option<String>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub set_pending_repo_switch: WriteSignal<Option<String>>,
    pub pending_repo_switch_nonce: ReadSignal<Option<u64>>,
    pub set_pending_repo_switch_nonce: WriteSignal<Option<u64>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub set_current_scope_nonce: WriteSignal<u64>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub set_shadow_repos: WriteSignal<Vec<String>>,
    pub shadow_list_request_id: ReadSignal<Option<String>>,
    pub set_shadow_list_request_id: WriteSignal<Option<String>>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub set_repo_list: WriteSignal<Vec<String>>,
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
        pending_branch_switch_nonce: scope.pending_branch_switch_nonce,
        set_pending_branch_switch_nonce: scope.set_pending_branch_switch_nonce,
        current_repo: scope.current_repo,
        set_current_repo: scope.set_current_repo,
        current_repo_id: scope.current_repo_id,
        set_current_repo_id: scope.set_current_repo_id,
        pending_repo_switch: scope.pending_repo_switch,
        set_pending_repo_switch: scope.set_pending_repo_switch,
        pending_repo_switch_nonce: scope.pending_repo_switch_nonce,
        set_pending_repo_switch_nonce: scope.set_pending_repo_switch_nonce,
        current_scope_nonce: scope.current_scope_nonce,
        set_current_scope_nonce: scope.set_current_scope_nonce,
        shadow_repos: projection.shadow_repos,
        set_shadow_repos: projection.set_shadow_repos,
        shadow_list_request_id: projection.shadow_list_request_id,
        set_shadow_list_request_id: projection.set_shadow_list_request_id,
        repo_list: projection.repo_list,
        set_repo_list: projection.set_repo_list,
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
