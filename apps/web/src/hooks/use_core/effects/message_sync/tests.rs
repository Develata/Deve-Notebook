use super::should_accept_sync_hello;
use deve_core::models::PeerId;

#[test]
fn ignores_sync_hello_while_viewing_remote_branch() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!should_accept_sync_hello(
        Some(repo_id.clone()),
        Some(PeerId::new("peer-a")),
        None,
        None,
        Some(3),
        &repo_id,
        3,
    ));
}

#[test]
fn ignores_sync_hello_while_pending_shadow_switch() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!should_accept_sync_hello(
        Some(repo_id.clone()),
        None,
        Some(crate::hooks::use_core::PendingBranchTarget::Shadow(
            "peer-a".into(),
        )),
        None,
        Some(3),
        &repo_id,
        3,
    ));
}

#[test]
fn ignores_sync_hello_while_pending_local_switch() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!should_accept_sync_hello(
        Some(repo_id.clone()),
        Some(PeerId::new("peer-a")),
        Some(crate::hooks::use_core::PendingBranchTarget::Local),
        None,
        Some(3),
        &repo_id,
        3,
    ));
}

#[test]
fn ignores_sync_hello_while_pending_repo_switch() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!should_accept_sync_hello(
        Some(repo_id.clone()),
        None,
        None,
        Some("test".into()),
        Some(3),
        &repo_id,
        3,
    ));
}

#[test]
fn ignores_sync_hello_with_stale_scope_nonce() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!should_accept_sync_hello(
        Some(repo_id.clone()),
        None,
        None,
        None,
        Some(4),
        &repo_id,
        3,
    ));
}
