use deve_core::models::PeerId;
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
    pub on_switch_repo: Callback<String>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,
    pub repo_list: ReadSignal<Vec<String>>,
}
