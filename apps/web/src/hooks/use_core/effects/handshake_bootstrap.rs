use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::Set;

use super::super::types::HandshakeSignals;

pub(super) fn restore_session_scope(
    ws: &WsService,
    signals: HandshakeSignals,
    current_repo: Option<String>,
    active_branch: Option<PeerId>,
) {
    if let Some(branch) = active_branch {
        ws.send(ClientMessage::SwitchBranch {
            peer_id: Some(branch.to_string()),
        });
        if let Some(repo_name) = current_repo {
            ws.send(ClientMessage::SwitchRepo { name: repo_name });
        } else {
            request_repo_list(ws, signals);
        }
        return;
    }

    if let Some(repo_name) = current_repo {
        ws.send(ClientMessage::SwitchRepo { name: repo_name });
        request_repo_list(ws, signals);
        return;
    }

    ws.send(ClientMessage::ListDocs);
    request_repo_list(ws, signals);
}

fn request_repo_list(ws: &WsService, signals: HandshakeSignals) {
    let request_id = uuid::Uuid::new_v4().to_string();
    signals.set_repo_list_request_id.set(Some(request_id.clone()));
    ws.send(ClientMessage::ListRepos { request_id });
}
