use super::*;

#[test]
fn read_list_dispatch_accepts_changes_remote_branch_scope() {
    let repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_read_list(
        ReadListKind::Changes,
        repo_id,
        Some(branch.clone()),
        Some(branch),
        Some(17),
    );

    assert!(result.handled);
    assert_eq!(result.changes_request_id, None);
    assert_eq!(result.notice, None);
    assert_eq!(result.staged[0].path, "fresh-staged.md");
    assert_eq!(result.unstaged[0].path, "fresh-unstaged.md");
    assert_eq!(result.history_request_id.as_deref(), Some("history-req-1"));
    assert_eq!(result.history[0].id, "stale");
}

#[test]
fn read_list_dispatch_rejects_changes_stale_scope_nonce() {
    let repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_read_list(
        ReadListKind::Changes,
        repo_id,
        Some(branch.clone()),
        Some(branch),
        Some(16),
    );

    assert!(result.handled);
    assert_changes_preserved(&result);
}

#[test]
fn read_list_dispatch_rejects_changes_other_branch() {
    let repo_id = uuid::Uuid::new_v4();
    let result = dispatch_read_list(
        ReadListKind::Changes,
        repo_id,
        Some(PeerId::new("peer-a")),
        Some(PeerId::new("peer-b")),
        Some(17),
    );

    assert!(result.handled);
    assert_changes_preserved(&result);
}

#[test]
fn read_list_dispatch_rejects_changes_other_repo() {
    let current_repo_id = uuid::Uuid::new_v4();
    let message_repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_read_list_from_repo(
        ReadListKind::Changes,
        current_repo_id,
        message_repo_id,
        Some(branch.clone()),
        Some(branch),
        Some(17),
    );

    assert!(result.handled);
    assert_changes_preserved(&result);
}
