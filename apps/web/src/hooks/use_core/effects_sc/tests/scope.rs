use super::*;

#[test]
fn ignores_repo_scoped_messages_before_repo_scope_is_ready() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (current_repo_id, _) = signal(None::<String>);
    assert!(!matches_current_repo(
        &Some(uuid::Uuid::new_v4()),
        current_repo_id,
        None,
    ));
    assert!(matches_current_repo(&None, current_repo_id, None));
}

#[test]
fn rejects_repo_less_sc_messages_once_repo_scope_is_ready() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (current_repo_id, _) = signal(Some(uuid::Uuid::new_v4().to_string()));
    assert!(!matches_current_repo(&None, current_repo_id, None));
}

#[test]
fn ignores_repo_scoped_messages_from_other_branch() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    assert!(!matches_current_scope(
        &Some(repo_id),
        &Some(PeerId::new("peer-b")),
        current_repo_id,
        active_branch,
        pending_branch_switch,
        pending_repo_switch,
    ));
}

#[test]
fn rejects_repo_scoped_messages_while_repo_switch_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    let (active_branch, _) = signal(None::<PeerId>);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(Some(PendingRepoSwitch::switch("test", 1)));
    assert!(!matches_current_scope(
        &Some(repo_id),
        &None,
        current_repo_id,
        active_branch,
        pending_branch_switch,
        pending_repo_switch,
    ));
}
