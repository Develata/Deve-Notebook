//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
use crate::runtime::domain::{RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest};
use crate::runtime::repo_control_client::RepoRemovalPresentation;
use deve_core::models::PeerId;
use deve_core::protocol::RepoListEntry;
use leptos::prelude::*;

#[derive(Clone)]
pub struct BranchContext {
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub on_switch_branch: Callback<Option<String>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub set_current_repo_id: WriteSignal<Option<String>>,
    pub on_switch_repo: Callback<RepoSwitchRequest>,
    pub on_create_repo: Callback<String>,
    pub on_rename_repo: Callback<RepoRenameRequest>,
    pub on_remove_repo: Callback<RepoRemoveRequest>,
    pub removal_preview: ReadSignal<Option<RepoRemovalPresentation>>,
    pub on_confirm_remove_repo: Callback<deve_core::models::RepoId>,
    pub on_cancel_remove_repo: Callback<deve_core::models::RepoId>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub repo_entries: ReadSignal<Vec<RepoListEntry>>,
}
