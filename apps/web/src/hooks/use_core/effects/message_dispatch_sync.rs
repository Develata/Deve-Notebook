//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use deve_core::models::{PeerId, RepoId, VersionVector};

use super::super::state::CoreSignals;
use super::message_runtime_sync::{
    handle_merge_complete, handle_pending_discarded, handle_pending_ops_info,
    handle_sync_mode_status,
};
use super::message_sync::handle_sync_hello;
use crate::runtime::domain::PendingOpsPreview;

pub fn handle_sync_hello_message(
    peer_id: PeerId,
    repo_id: RepoId,
    scope_nonce: u64,
    vector: VersionVector,
    signals: CoreSignals,
) {
    handle_sync_hello(peer_id, repo_id.to_string(), scope_nonce, vector, signals);
}

pub fn handle_sync_mode_status_message(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    mode: String,
    signals: CoreSignals,
) {
    handle_sync_mode_status(request_id, repo_id, branch, scope_nonce, mode, signals);
}

pub fn handle_pending_ops_info_message(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    count: u32,
    previews: Vec<PendingOpsPreview>,
    signals: CoreSignals,
) {
    handle_pending_ops_info(
        request_id,
        repo_id,
        branch,
        scope_nonce,
        count,
        previews,
        signals,
    );
}

pub fn handle_merge_complete_message(
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    merged_count: u32,
    signals: CoreSignals,
) {
    handle_merge_complete(repo_id, branch, scope_nonce, merged_count, signals);
}

pub fn handle_pending_discarded_message(
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    signals: CoreSignals,
) {
    handle_pending_discarded(repo_id, branch, scope_nonce, signals);
}
