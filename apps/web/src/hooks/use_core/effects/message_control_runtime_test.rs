use super::message_control_runtime_repo::{next_request_id, should_request_repo_sync_state};
use deve_core::protocol::ClientMessage;

#[test]
fn repo_sync_state_requests_only_run_on_local_branch() {
    assert!(should_request_repo_sync_state(None));
    assert!(!should_request_repo_sync_state(Some(
        deve_core::models::PeerId::new("peer-a")
    )));
}

#[test]
fn request_ids_are_non_empty() {
    let request_id = next_request_id();
    assert!(!request_id.is_empty());
    assert!(uuid::Uuid::parse_str(&request_id).is_ok());
}

#[test]
fn list_repos_request_keeps_shared_request_id_shape() {
    let request_id = next_request_id();
    let msg = ClientMessage::ListRepos {
        request_id: request_id.clone(),
        scope_nonce: Some(7),
    };
    assert!(matches!(
        msg,
        ClientMessage::ListRepos {
            request_id: actual,
            scope_nonce: Some(7),
        } if actual == request_id
    ));
}
