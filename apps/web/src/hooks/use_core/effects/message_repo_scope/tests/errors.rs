use super::*;

#[test]
fn rejects_edit_rejected_while_scope_switch_pending_or_nonce_is_stale() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(status);
    signals.set_current_scope_nonce.set(7);
    signals
        .set_pending_branch_switch
        .set(Some(PendingBranchSwitch::new(
            PendingBranchTarget::Local,
            3,
        )));
    assert!(!accepts_edit_rejected_message(Some(7), signals));
    signals.set_pending_branch_switch.set(None);
    signals
        .set_pending_repo_switch
        .set(Some(PendingRepoSwitch::switch(
            "wiki",
            uuid::Uuid::nil(),
            3,
        )));
    assert!(!accepts_edit_rejected_message(Some(7), signals));
    signals.set_pending_repo_switch.set(None);
    assert!(!accepts_edit_rejected_message(Some(9), signals));
    assert!(!accepts_edit_rejected_message(None, signals));
    assert!(accepts_edit_rejected_message(Some(7), signals));
}

#[test]
fn rejects_scoped_protocol_errors_while_scope_switch_pending_or_nonce_is_stale() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(status);
    signals.set_current_scope_nonce.set(7);
    signals
        .set_pending_branch_switch
        .set(Some(PendingBranchSwitch::new(
            PendingBranchTarget::Local,
            3,
        )));
    assert!(!accepts_protocol_error_message(Some(7), None, signals));
    signals.set_pending_branch_switch.set(None);
    signals
        .set_pending_repo_switch
        .set(Some(PendingRepoSwitch::switch(
            "wiki",
            uuid::Uuid::nil(),
            3,
        )));
    assert!(!accepts_protocol_error_message(Some(7), None, signals));
    signals.set_pending_repo_switch.set(None);
    assert!(!accepts_protocol_error_message(Some(9), None, signals));
    assert!(!accepts_protocol_error_message(None, None, signals));
    assert!(!accepts_protocol_error_message(None, Some(3), signals));
    signals
        .set_pending_repo_switch
        .set(Some(PendingRepoSwitch::switch(
            "wiki",
            uuid::Uuid::nil(),
            3,
        )));
    assert!(accepts_protocol_error_message(None, Some(3), signals));
    signals.set_pending_repo_switch.set(None);
    assert!(accepts_protocol_error_message(Some(7), None, signals));
}
