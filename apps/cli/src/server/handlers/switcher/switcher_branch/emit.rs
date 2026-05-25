//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Branch switch success event emission.

use super::super::switcher_payload::{RepoViewMessages, emit_repo_view};
use crate::server::channel::DualChannel;
use deve_core::protocol::ServerMessage;

pub(super) fn emit_branch_switch_messages(
    ch: &DualChannel,
    final_branch: Option<String>,
    scope_nonce: Option<u64>,
    switch_nonce: Option<u64>,
    repos: Vec<String>,
    repo_view: Option<RepoViewMessages>,
) {
    ch.unicast(ServerMessage::BranchSwitched {
        peer_id: final_branch.clone(),
        success: true,
        switch_nonce,
    });
    ch.unicast(ServerMessage::RepoList {
        request_id: None,
        branch: final_branch,
        scope_nonce,
        repos,
    });
    emit_repo_view(ch, repo_view);
}
