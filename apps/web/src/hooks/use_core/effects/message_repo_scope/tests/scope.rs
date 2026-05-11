use super::*;

#[test]
fn rejects_repo_scoped_messages_while_repo_switch_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    assert!(!matches_repo_scope(
        &Some(repo_id),
        &None,
        current_repo_id,
        None,
        None,
        Some("test".into()),
    ));
}

#[test]
fn rejects_repo_scoped_messages_while_branch_switch_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    assert!(!matches_repo_scope(
        &Some(repo_id),
        &Some(PeerId::new("peer-a")),
        current_repo_id,
        None,
        Some(PendingBranchTarget::Shadow("peer-a".into())),
        None,
    ));
}

#[test]
fn projection_messages_accept_exact_current_repo_during_repo_switch_settle() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(status);
    let repo_id = uuid::Uuid::new_v4();
    signals.set_current_repo_id.set(Some(repo_id.to_string()));
    signals.set_pending_repo_switch.set(Some("default".into()));

    assert!(matches_projection_message_scope(
        &Some(repo_id),
        &None,
        signals,
    ));
    assert!(!matches_repo_scope(
        &Some(repo_id),
        &None,
        signals.current_repo_id,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
    ));
}
