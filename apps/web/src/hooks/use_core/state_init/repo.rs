use crate::storage::DegradedSyncMode;
use deve_core::models::PeerId;
use deve_core::tree::FileNode;
use leptos::prelude::*;

use super::super::types::PendingBranchTarget;

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
    let (active_branch, set_active_branch) = signal(None::<PeerId>);
    let (pending_branch_switch, set_pending_branch_switch) = signal(None::<PendingBranchTarget>);
    let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(None::<u64>);
    let (current_repo, set_current_repo) = signal(None::<String>);
    let (current_repo_id, set_current_repo_id) = signal(None::<String>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<String>);
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(None::<u64>);
    let (current_scope_nonce, set_current_scope_nonce) = signal(0u64);
    let (shadow_repos, set_shadow_repos) = signal(Vec::new());
    let (shadow_list_request_id, set_shadow_list_request_id) = signal(None::<String>);
    let (repo_list, set_repo_list) = signal(Vec::new());
    let (repo_list_request_id, set_repo_list_request_id) = signal(None::<String>);
    let (doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
    let (tree_request_id, set_tree_request_id) = signal(None::<String>);
    let (tree_nodes, set_tree_nodes) = signal(Vec::<FileNode>::new());
    let (degraded_sync_mode, set_degraded_sync_mode) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);

    RepoSignals {
        active_branch,
        set_active_branch,
        pending_branch_switch,
        set_pending_branch_switch,
        pending_branch_switch_nonce,
        set_pending_branch_switch_nonce,
        current_repo,
        set_current_repo,
        current_repo_id,
        set_current_repo_id,
        pending_repo_switch,
        set_pending_repo_switch,
        pending_repo_switch_nonce,
        set_pending_repo_switch_nonce,
        current_scope_nonce,
        set_current_scope_nonce,
        shadow_repos,
        set_shadow_repos,
        shadow_list_request_id,
        set_shadow_list_request_id,
        repo_list,
        set_repo_list,
        repo_list_request_id,
        set_repo_list_request_id,
        doc_list_request_id,
        set_doc_list_request_id,
        tree_request_id,
        set_tree_request_id,
        tree_nodes,
        set_tree_nodes,
        degraded_sync_mode,
        set_degraded_sync_mode,
        sync_banner,
        set_sync_banner,
    }
}
