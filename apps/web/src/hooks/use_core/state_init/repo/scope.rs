//! plan_ref:
//!   - 03_storage/index#browser-storage-layering
//!   - 04_repository#repo-scope-runtime
//!
use crate::runtime::repo_control_client::RepoRemovalPresentation;
use crate::storage::DegradedSyncMode;
use deve_core::models::PeerId;
use leptos::prelude::*;

use super::super::super::types::{PendingBranchSwitch, PendingRepoSwitch};

#[derive(Clone, Copy)]
pub(super) struct RepoScopeSignals {
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
    pub removal_preview: ReadSignal<Option<RepoRemovalPresentation>>,
    pub set_removal_preview: WriteSignal<Option<RepoRemovalPresentation>>,
    pub explicit_repo_selection_required: ReadSignal<bool>,
    pub set_explicit_repo_selection_required: WriteSignal<bool>,
    pub degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    pub set_degraded_sync_mode: WriteSignal<Option<DegradedSyncMode>>,
    pub sync_banner: ReadSignal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
}

pub(super) fn init_repo_scope_signals() -> RepoScopeSignals {
    let (active_branch, set_active_branch) = signal(None::<PeerId>);
    let (pending_branch_switch, set_pending_branch_switch) = signal(None::<PendingBranchSwitch>);
    let (current_repo, set_current_repo) = signal(None::<String>);
    let (current_repo_id, set_current_repo_id) = signal(None::<String>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
    let (current_scope_nonce, set_current_scope_nonce) = signal(0u64);
    let (removal_preview, set_removal_preview) = signal(None::<RepoRemovalPresentation>);
    let (explicit_repo_selection_required, set_explicit_repo_selection_required) = signal(false);
    let (degraded_sync_mode, set_degraded_sync_mode) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);

    RepoScopeSignals {
        active_branch,
        set_active_branch,
        pending_branch_switch,
        set_pending_branch_switch,
        current_repo,
        set_current_repo,
        current_repo_id,
        set_current_repo_id,
        pending_repo_switch,
        set_pending_repo_switch,
        current_scope_nonce,
        set_current_scope_nonce,
        removal_preview,
        set_removal_preview,
        explicit_repo_selection_required,
        set_explicit_repo_selection_required,
        degraded_sync_mode,
        set_degraded_sync_mode,
        sync_banner,
        set_sync_banner,
    }
}
