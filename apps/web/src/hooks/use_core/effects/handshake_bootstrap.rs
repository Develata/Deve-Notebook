use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set};

use super::super::types::HandshakeSignals;
use super::super::{PendingBranchTarget, switch_nonce::next_switch_nonce};

pub(super) fn restore_session_scope(
    ws: &WsService,
    signals: HandshakeSignals,
    current_repo: Option<String>,
    current_repo_id: Option<String>,
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
        let switch_nonce = next_switch_nonce();
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

    request_doc_listing(ws, signals);
    request_repo_list(ws, signals);
}

fn request_repo_list(ws: &WsService, signals: HandshakeSignals) {
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_repo_list_request_id
        .set(Some(request_id.clone()));
    ws.send(ClientMessage::ListRepos {
        request_id,
        scope_nonce: Some(signals.current_scope_nonce.get_untracked()),
    });
}

fn request_doc_listing(ws: &WsService, signals: HandshakeSignals) {
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_doc_list_request_id
        .set(Some(request_id.clone()));
    signals.set_tree_request_id.set(Some(request_id.clone()));
    ws.send(ClientMessage::ListDocs {
        request_id,
        scope_nonce: Some(signals.current_scope_nonce.get_untracked()),
    });
}

fn build_switch_repo(name: String, repo_id: Option<String>, switch_nonce: u64) -> Option<ClientMessage> {
    match repo_id {
        Some(repo_id) => match uuid::Uuid::parse_str(&repo_id) {
            Ok(repo_id) => Some(ClientMessage::SwitchRepoExact {
                name,
                repo_id,
                switch_nonce: Some(switch_nonce),
            }),
            Err(_) => {
                leptos::logging::warn!(
                    "拒绝按损坏的 repo_id 恢复仓库作用域: name={}, repo_id={}",
                    name,
                    repo_id
                );
                None
            }
        },
        None => Some(ClientMessage::SwitchRepo {
            name,
            switch_nonce: Some(switch_nonce),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::build_switch_repo;
    use deve_core::protocol::ClientMessage;

    #[test]
    fn build_switch_repo_uses_exact_variant_for_uuid() {
        let repo_id = uuid::Uuid::new_v4();
        let msg = build_switch_repo("default".into(), Some(repo_id.to_string()), 9)
            .expect("valid repo id should restore exactly");
        assert!(matches!(
            msg,
            ClientMessage::SwitchRepoExact {
                name,
                repo_id: actual,
                switch_nonce: Some(9),
            } if name == "default" && actual == repo_id
        ));
    }

    #[test]
    fn build_switch_repo_rejects_invalid_uuid() {
        assert!(build_switch_repo("default".into(), Some("not-a-uuid".into()), 7).is_none());
    }

    #[test]
    fn build_switch_repo_uses_name_only_when_repo_id_missing() {
        let msg = build_switch_repo("default".into(), None, 7);
        assert!(matches!(
            msg,
            Some(ClientMessage::SwitchRepo {
                name,
                switch_nonce: Some(7),
            }) if name == "default"
        ));
    }
}
