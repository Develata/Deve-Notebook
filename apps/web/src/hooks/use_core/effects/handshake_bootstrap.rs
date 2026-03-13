use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::Set;

use super::super::types::HandshakeSignals;
use super::super::{PendingBranchTarget, switch_nonce::next_switch_nonce};

pub(super) fn restore_session_scope(
    ws: &WsService,
    signals: HandshakeSignals,
    current_repo: Option<String>,
    active_branch: Option<PeerId>,
) {
    if let Some(branch) = active_branch {
        let switch_nonce = next_switch_nonce();
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
        if let Some(repo_name) = current_repo {
            signals.set_pending_repo_switch.set(Some(repo_name.clone()));
            signals
                .set_pending_repo_switch_nonce
                .set(Some(switch_nonce));
            ws.send(ClientMessage::SwitchRepo {
                name: repo_name,
                switch_nonce: Some(switch_nonce),
            });
        } else {
            request_repo_list(ws, signals);
        }
        return;
    }

    if let Some(repo_name) = current_repo {
        let switch_nonce = next_switch_nonce();
        signals.set_pending_repo_switch.set(Some(repo_name.clone()));
        signals
            .set_pending_repo_switch_nonce
            .set(Some(switch_nonce));
        ws.send(ClientMessage::SwitchRepo {
            name: repo_name,
            switch_nonce: Some(switch_nonce),
        });
        request_repo_list(ws, signals);
        return;
    }

    request_doc_listing(ws, signals);
    request_repo_list(ws, signals);
}

fn request_repo_list(ws: &WsService, signals: HandshakeSignals) {
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_repo_list_request_id
        .set(Some(request_id.clone()));
    ws.send(ClientMessage::ListRepos { request_id });
}

fn request_doc_listing(ws: &WsService, signals: HandshakeSignals) {
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_doc_list_request_id
        .set(Some(request_id.clone()));
    signals.set_tree_request_id.set(Some(request_id.clone()));
    ws.send(ClientMessage::ListDocs { request_id });
}
