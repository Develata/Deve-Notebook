//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::storage::identity::{note_handshake, save_repo_vector};
use deve_core::models::{PeerId, VersionVector};
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::super::state::CoreSignals;
use super::super::types::PeerSession;
use super::message_scope::peer_branch_matches_scope;

pub fn handle_sync_hello(
    peer_id: PeerId,
    repo_id: String,
    scope_nonce: u64,
    vector: VersionVector,
    signals: CoreSignals,
) {
    let accepted = should_accept_sync_hello(
        signals.current_repo_id.get_untracked(),
        signals.active_branch.get_untracked(),
        signals
            .pending_branch_switch
            .get_untracked()
            .map(|pending| pending.into_target()),
        signals
            .pending_repo_switch
            .get_untracked()
            .map(|pending| pending.expected_name),
        signals.handshake_scope_nonce.get_untracked(),
        &repo_id,
        scope_nonce,
    );
    if accepted {
        signals.set_handshake_ready.set(true);
    }
    if !accepted {
        return;
    }
    signals.set_peers.update(|map| {
        map.insert(
            peer_id.clone(),
            PeerSession {
                id: peer_id,
                vector: vector.clone(),
                last_seen: js_sys::Date::now() as u64,
            },
        );
    });
    spawn_local(async move {
        match serde_json::to_string(&vector) {
            Ok(vector_json) => {
                let _ = save_repo_vector(&repo_id, &vector_json).await;
            }
            Err(err) => leptos::logging::warn!("保存 repo 向量失败: {}", err),
        }
        let _ = note_handshake(&repo_id).await;
    });
}

fn should_accept_sync_hello(
    current_repo_id: Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<crate::hooks::use_core::PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    handshake_scope_nonce: Option<u64>,
    repo_id: &str,
    scope_nonce: u64,
) -> bool {
    pending_repo_switch.is_none()
        && pending_branch_switch.is_none()
        && handshake_scope_nonce == Some(scope_nonce)
        && peer_branch_matches_scope(&None, active_branch, pending_branch_switch)
        && current_repo_id.as_deref() == Some(repo_id)
}

#[cfg(test)]
mod tests;
