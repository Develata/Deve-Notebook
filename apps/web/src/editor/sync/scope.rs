//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::{PeerId, RepoId};

#[derive(Clone)]
pub struct SyncPayloadScope {
    pub current_repo_id: Option<String>,
    pub pending_repo_switch: Option<String>,
    pub current_branch: Option<PeerId>,
    pub pending_branch_switch: Option<PendingBranchTarget>,
    pub handshake_scope_nonce: Option<u64>,
}

#[derive(Clone)]
pub struct ScopedMessageScope {
    pub current_repo_id: Option<String>,
    pub pending_repo_switch: Option<String>,
    pub current_branch: Option<PeerId>,
    pub pending_branch_switch: Option<PendingBranchTarget>,
    pub current_scope_nonce: u64,
}

pub fn matches_scope(
    current_repo_id: Option<String>,
    pending_repo_switch: Option<String>,
    current_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
) -> bool {
    if pending_repo_switch.is_some() || pending_branch_switch.is_some() {
        return false;
    }
    let expected_branch = pending_branch_switch
        .map(|pending| match pending {
            PendingBranchTarget::Local => None,
            PendingBranchTarget::Shadow(peer_id) => Some(PeerId::new(peer_id)),
        })
        .unwrap_or(current_branch);
    match (repo_id, current_repo_id) {
        (Some(repo_id), Some(current)) => {
            current == repo_id.to_string() && branch == expected_branch
        }
        (Some(_), None) => false,
        (None, Some(_)) => false,
        (None, None) => branch == expected_branch,
    }
}

pub fn accepts_sync_payload(
    scope: SyncPayloadScope,
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: u64,
) -> bool {
    scope.handshake_scope_nonce == Some(scope_nonce)
        && matches_scope(
            scope.current_repo_id,
            scope.pending_repo_switch,
            scope.current_branch,
            scope.pending_branch_switch,
            Some(repo_id),
            branch,
        )
}

pub fn matches_scoped_message(
    scope: ScopedMessageScope,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
) -> bool {
    scope_nonce == Some(scope.current_scope_nonce)
        && matches_scope(
            scope.current_repo_id,
            scope.pending_repo_switch,
            scope.current_branch,
            scope.pending_branch_switch,
            repo_id,
            branch,
        )
}

#[cfg(test)]
#[path = "scope_test.rs"]
mod tests;
