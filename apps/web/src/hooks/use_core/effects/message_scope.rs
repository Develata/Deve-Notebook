//! 前端 repo-scoped 请求/响应门禁。
//!
//! Invariants:
//! - repo-scoped request/response 必须匹配当前 `scope_nonce`。
//! - branch/repo 切换挂起期间，不得接受任何 repo-scoped 列表刷新。
//! - system push 与 request-response 都必须落在当前 scope 代际内。

use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

#[path = "message_scope_branch.rs"]
mod branch;
use self::branch::{expected_branch_string, expected_peer_branch};

#[derive(Clone, Copy)]
pub struct RequestMatch<'a> {
    pub message_id: Option<&'a str>,
    pub expected_id: Option<&'a str>,
    pub scope_nonce: Option<u64>,
    pub current_scope_nonce: u64,
}

#[derive(Clone)]
pub struct RepoListScope {
    pub active_branch: Option<PeerId>,
    pub pending_branch_switch: Option<PendingBranchTarget>,
    pub pending_repo_switch: Option<String>,
}

#[derive(Clone)]
pub struct ShadowListScope {
    pub pending_branch_switch: Option<PendingBranchTarget>,
    pub pending_repo_switch: Option<String>,
}

pub fn peer_branch_matches_scope(
    branch: &Option<PeerId>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> bool {
    *branch == expected_peer_branch(active_branch, pending_branch_switch)
}

pub fn string_branch_matches_scope(
    branch: &Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> bool {
    *branch == expected_branch_string(active_branch, pending_branch_switch)
}

pub fn repo_list_matches_scope(
    request: RequestMatch<'_>,
    branch: Option<String>,
    scope: &RepoListScope,
) -> bool {
    request_matches(request)
        && scope.pending_repo_switch.is_none()
        && scope.pending_branch_switch.is_none()
        && branch
            == expected_branch_string(
                scope.active_branch.clone(),
                scope.pending_branch_switch.clone(),
            )
}

pub fn shadow_list_matches_scope(request: RequestMatch<'_>, scope: &ShadowListScope) -> bool {
    request_matches(request)
        && scope.pending_branch_switch.is_none()
        && scope.pending_repo_switch.is_none()
}

pub fn accepts_system_or_matching_request(
    message_id: Option<&str>,
    expected_id: Option<&str>,
    scope_nonce: Option<u64>,
    current_scope_nonce: u64,
) -> bool {
    request_matches(RequestMatch {
        message_id,
        expected_id,
        scope_nonce,
        current_scope_nonce,
    })
}

fn request_matches(request: RequestMatch<'_>) -> bool {
    let scoped_match = request.scope_nonce == Some(request.current_scope_nonce);
    match request.message_id {
        Some(message_id) => request.expected_id == Some(message_id) && scoped_match,
        None => {
            request.expected_id.is_none()
                && request.scope_nonce == Some(request.current_scope_nonce)
        }
    }
}

#[cfg(test)]
#[path = "message_scope_test.rs"]
mod tests;
