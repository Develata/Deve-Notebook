use crate::api::WsService;
use leptos::prelude::{GetUntracked, Set};

use super::super::state::CoreSignals;
use super::message_control;
use super::message_repo_bootstrap::maybe_switch_to_first_repo;
use super::message_scope::{
    RepoListScope, RequestMatch, accepts_system_or_matching_request, repo_list_matches_scope,
};
use super::message_shadow;

pub fn handle_repo_list_message(
    request_id: Option<String>,
    branch: Option<String>,
    scope_nonce: Option<u64>,
    repos: Vec<String>,
    ws: &WsService,
    signals: CoreSignals,
) {
    if !repo_list_matches_scope(
        RequestMatch {
            message_id: request_id.as_deref(),
            expected_id: signals.repo_list_request_id.get_untracked().as_deref(),
            scope_nonce,
            current_scope_nonce: signals.current_scope_nonce.get_untracked(),
        },
        branch,
        &RepoListScope {
            active_branch: signals.active_branch.get_untracked(),
            pending_branch_switch: signals.pending_branch_switch.get_untracked(),
            pending_repo_switch: signals.pending_repo_switch.get_untracked(),
        },
    ) {
        return;
    }

    if request_id.is_some() {
        signals.set_repo_list_request_id.set(None);
    }
    signals.set_repo_list.set(repos.clone());
    maybe_switch_to_first_repo(&repos, ws, signals);
}

pub fn handle_peer_deleted_message(
    peer_id: String,
    scope_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    if !accepts_system_or_matching_request(
        None,
        None,
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
    ) {
        return;
    }
    message_shadow::handle_peer_deleted(peer_id, ws, signals);
}

pub fn handle_branch_switched_message(
    peer_id: Option<String>,
    success: bool,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    message_control::handle_branch_switched(peer_id, success, switch_nonce, ws, signals);
}

pub fn handle_repo_switched_message(
    branch: Option<String>,
    name: String,
    uuid: String,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    message_control::handle_repo_switched(branch, name, uuid, switch_nonce, ws, signals);
}
