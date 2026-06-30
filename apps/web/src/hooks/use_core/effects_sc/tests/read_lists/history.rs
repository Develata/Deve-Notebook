use super::*;

#[test]
fn read_list_dispatch_accepts_history_remote_branch_scope() {
    let repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_read_list(
        ReadListKind::History,
        repo_id,
        Some(branch.clone()),
        Some(branch),
        Some(17),
    );

    assert!(result.handled);
    assert_eq!(result.history_request_id, None);
    assert_eq!(result.notice, None);
    assert_eq!(result.history[0].id, "fresh");
    assert_eq!(result.changes_request_id.as_deref(), Some("changes-req-1"));
    assert_eq!(result.staged[0].path, "stale-staged.md");
    assert_eq!(result.unstaged[0].path, "stale-unstaged.md");
}

#[test]
fn history_list_keeps_local_command_notice() {
    let repo_id = uuid::Uuid::new_v4();
    let result = dispatch_read_list_from_repo_with_notice(
        ReadListKind::History,
        repo_id,
        repo_id,
        None,
        None,
        Some(17),
        SourceControlNotice::git_push_cli_only(),
    );

    assert!(result.handled);
    assert_eq!(result.history_request_id, None);
    assert_eq!(
        result.notice,
        Some(SourceControlNotice::git_push_cli_only())
    );
    assert_eq!(result.history[0].id, "fresh");
}

#[test]
fn read_list_dispatch_rejects_history_stale_scope_nonce() {
    let repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_read_list(
        ReadListKind::History,
        repo_id,
        Some(branch.clone()),
        Some(branch),
        Some(16),
    );

    assert!(result.handled);
    assert_history_preserved(&result);
}

#[test]
fn read_list_dispatch_rejects_history_other_branch() {
    let repo_id = uuid::Uuid::new_v4();
    let result = dispatch_read_list(
        ReadListKind::History,
        repo_id,
        Some(PeerId::new("peer-a")),
        Some(PeerId::new("peer-b")),
        Some(17),
    );

    assert!(result.handled);
    assert_history_preserved(&result);
}

#[test]
fn read_list_dispatch_rejects_history_other_repo() {
    let current_repo_id = uuid::Uuid::new_v4();
    let message_repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_read_list_from_repo(
        ReadListKind::History,
        current_repo_id,
        message_repo_id,
        Some(branch.clone()),
        Some(branch),
        Some(17),
    );

    assert!(result.handled);
    assert_history_preserved(&result);
}
