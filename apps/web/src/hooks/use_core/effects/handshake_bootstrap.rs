use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;

pub(super) fn restore_session_scope(
    ws: &WsService,
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
            ws.send(ClientMessage::ListRepos);
        }
        return;
    }

    if let Some(repo_name) = current_repo {
        ws.send(ClientMessage::SwitchRepo { name: repo_name });
        ws.send(ClientMessage::ListRepos);
        return;
    }

    ws.send(ClientMessage::ListDocs);
    ws.send(ClientMessage::ListRepos);
}
