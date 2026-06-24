//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 07_network#web-ws-runtime
//!
//! Browser scope client runtime.
//!
//! This adapter coordinates repo/branch scope and stale-scope recovery inputs.
//! Server/core remain the authority for repo identity and writable state.

use deve_core::models::{DocId, PeerId};
use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct ScopeClient {
    pub current_doc: ReadSignal<Option<DocId>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub on_switch_repo: Callback<String>,
    pub on_create_repo: Callback<String>,
    pub on_switch_branch: Callback<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub set_current_repo_id: WriteSignal<Option<String>>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub is_spectator: Signal<bool>,
}
