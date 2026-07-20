//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use leptos::prelude::{GetUntracked, Set};

use super::super::super::types::HandshakeSignals;
use deve_core::protocol::ClientMessage;

pub(super) fn request_repo_list(ws: &WsService, signals: HandshakeSignals) {
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_repo_list_request_id
        .set(Some(request_id.clone()));
    ws.send(ClientMessage::ListRepos {
        request_id,
        scope_nonce: Some(signals.current_scope_nonce.get_untracked()),
    });
}

pub(super) fn build_switch_repo(
    repo_id: Option<String>,
    switch_nonce: u64,
) -> Option<ClientMessage> {
    match repo_id {
        Some(repo_id) => match uuid::Uuid::parse_str(&repo_id) {
            Ok(repo_id) => Some(ClientMessage::SwitchRepoExact {
                repo_id,
                switch_nonce: Some(switch_nonce),
            }),
            Err(_) => {
                leptos::logging::warn!("拒绝按损坏的 repo_id 恢复仓库作用域: {repo_id}");
                None
            }
        },
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::build_switch_repo;
    use deve_core::protocol::ClientMessage;

    #[test]
    fn build_switch_repo_uses_exact_variant_for_uuid() {
        let repo_id = uuid::Uuid::new_v4();
        let msg = build_switch_repo(Some(repo_id.to_string()), 9)
            .expect("valid repo id should restore exactly");
        assert!(matches!(
            msg,
            ClientMessage::SwitchRepoExact {
                repo_id: actual,
                switch_nonce: Some(9),
            } if actual == repo_id
        ));
    }

    #[test]
    fn build_switch_repo_rejects_invalid_uuid() {
        assert!(build_switch_repo(Some("not-a-uuid".into()), 7).is_none());
    }

    #[test]
    fn build_switch_repo_rejects_missing_repo_id() {
        assert!(build_switch_repo(None, 7).is_none());
    }
}
