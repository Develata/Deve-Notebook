//! plan_ref:
//!   - 03_storage#browser-storage-layering
//!   - 04_repository#repo-scope-runtime
//!
use crate::storage::DegradedSyncMode;
use deve_core::models::PeerId;
use leptos::prelude::*;

use super::super::super::types::PendingBranchTarget;

#[derive(Clone, Copy)]
pub(super) struct RepoScopeSignals {
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
    pub degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    pub set_degraded_sync_mode: WriteSignal<Option<DegradedSyncMode>>,
    pub sync_banner: ReadSignal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
}

pub(super) fn init_repo_scope_signals() -> RepoScopeSignals {
    let (active_branch, set_active_branch) = signal(None::<PeerId>);
    let (pending_branch_switch, set_pending_branch_switch) = signal(None::<PendingBranchTarget>);
    let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(None::<u64>);
    let (current_repo, set_current_repo) = signal(None::<String>);
    let (current_repo_id, set_current_repo_id) = signal(None::<String>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<String>);
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(None::<u64>);
    let (current_scope_nonce, set_current_scope_nonce) = signal(0u64);
    let (degraded_sync_mode, set_degraded_sync_mode) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);

    RepoScopeSignals {
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
        degraded_sync_mode,
        set_degraded_sync_mode,
        sync_banner,
        set_sync_banner,
    }
}
