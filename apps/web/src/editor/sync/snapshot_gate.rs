use super::scope::matches_scope;
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::{PeerId, RepoId};
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
pub(super) struct SnapshotRequestGate {
    open_request_id: ReadSignal<u64>,
    current_repo_id: ReadSignal<Option<String>>,
    pending_repo_switch: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    current_scope_nonce: ReadSignal<u64>,
    session_generation: Arc<AtomicU64>,
    expected_generation: u64,
    repo_id: RepoId,
    branch: Option<PeerId>,
    request_id: u64,
    scope_nonce: u64,
}

#[derive(Clone)]
pub(super) struct SnapshotRequestMatch {
    pub open_request_id: u64,
    pub request_id: u64,
    pub current_repo_id: Option<String>,
    pub pending_repo_switch: Option<String>,
    pub active_branch: Option<PeerId>,
    pub pending_branch_switch: Option<PendingBranchTarget>,
    pub current_scope_nonce: u64,
    pub scope_nonce: u64,
    pub current_generation: u64,
    pub expected_generation: u64,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
}

impl SnapshotRequestGate {
    pub(super) fn new(
        open_request_id: ReadSignal<u64>,
        current_repo_id: ReadSignal<Option<String>>,
        pending_repo_switch: ReadSignal<Option<String>>,
        active_branch: ReadSignal<Option<PeerId>>,
        pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
        current_scope_nonce: ReadSignal<u64>,
        session_generation: Arc<AtomicU64>,
        expected_generation: u64,
        repo_id: RepoId,
        branch: Option<PeerId>,
        request_id: u64,
        scope_nonce: u64,
    ) -> Self {
        Self {
            open_request_id,
            current_repo_id,
            pending_repo_switch,
            active_branch,
            pending_branch_switch,
            current_scope_nonce,
            session_generation,
            expected_generation,
            repo_id,
            branch,
            request_id,
            scope_nonce,
        }
    }

    pub(super) fn matches(&self) -> bool {
        let current_generation = self.session_generation.load(Ordering::Relaxed);
        if current_generation != self.expected_generation {
            return false;
        }
        snapshot_request_matches(SnapshotRequestMatch {
            open_request_id: self.open_request_id.get_untracked(),
            request_id: self.request_id,
            current_repo_id: self.current_repo_id.get_untracked(),
            pending_repo_switch: self.pending_repo_switch.get_untracked(),
            active_branch: self.active_branch.get_untracked(),
            pending_branch_switch: self.pending_branch_switch.get_untracked(),
            current_scope_nonce: self.current_scope_nonce.get_untracked(),
            scope_nonce: self.scope_nonce,
            current_generation,
            expected_generation: self.expected_generation,
            repo_id: self.repo_id,
            branch: self.branch.clone(),
        })
    }
}

pub(super) fn snapshot_request_matches(args: SnapshotRequestMatch) -> bool {
    args.open_request_id == args.request_id
        && args.current_scope_nonce == args.scope_nonce
        && args.current_generation == args.expected_generation
        && matches_scope(
            args.current_repo_id,
            args.pending_repo_switch,
            args.active_branch,
            args.pending_branch_switch,
            Some(args.repo_id),
            args.branch,
        )
}
