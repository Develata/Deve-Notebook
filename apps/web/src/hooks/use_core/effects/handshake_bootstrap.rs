//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set};

use super::super::types::HandshakeSignals;
use super::super::{PendingBranchTarget, switch_nonce::next_switch_nonce_after};
mod repo;
use self::repo::{build_switch_repo, request_repo_list};

pub(super) fn restore_session_scope(
    ws: &WsService,
    signals: HandshakeSignals,
    current_repo: Option<String>,
    current_repo_id: Option<String>,
    active_branch: Option<PeerId>,
) {
    if let Some(branch) = active_branch {
        let Some(switch_nonce) =
            next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
        else {
            request_repo_list(ws, signals);
            return;
        };
        signals
            .set_pending_branch_switch
            .set(Some(PendingBranchTarget::Shadow(branch.to_string())));
        signals
            .set_pending_branch_switch_nonce
            .set(Some(switch_nonce));
        ws.send(ClientMessage::SwitchBranch {
            peer_id: Some(branch.to_string()),
            switch_nonce: Some(switch_nonce),
        });
        if let Some(repo_name) = current_repo
            && let Some(msg) =
                build_switch_repo(repo_name.clone(), current_repo_id.clone(), switch_nonce)
        {
            signals.set_pending_repo_switch.set(Some(repo_name));
            signals
                .set_pending_repo_switch_nonce
                .set(Some(switch_nonce));
            ws.send(msg);
        }
        return;
    }

    if let Some(repo_name) = current_repo {
        let Some(switch_nonce) =
            next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
        else {
            request_repo_list(ws, signals);
            return;
        };
        if let Some(msg) = build_switch_repo(repo_name.clone(), current_repo_id, switch_nonce) {
            signals.set_pending_repo_switch.set(Some(repo_name));
            signals
                .set_pending_repo_switch_nonce
                .set(Some(switch_nonce));
            ws.send(msg);
            return;
        }
        request_repo_list(ws, signals);
        return;
    }

    request_repo_list(ws, signals);
}
