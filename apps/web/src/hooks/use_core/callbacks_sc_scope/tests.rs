use super::{source_control_read_scope_nonce, source_control_scope_nonce};
use crate::hooks::use_core::{
    PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch,
    callbacks_sc::SourceControlScopeSignals,
};
use leptos::prelude::*;

#[test]
fn source_control_scope_requires_bound_repo_and_no_pending_switch() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (current_repo_id, _) = signal(None::<String>);
    let (active_branch, _) = signal(None::<deve_core::models::PeerId>);
    let (current_scope_nonce, _) = signal(7u64);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    assert_eq!(
        source_control_scope_nonce(SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        }),
        None
    );
    let (current_repo_id, _) = signal(Some("repo-a".to_string()));
    let (pending_branch_switch, _) = signal(Some(PendingBranchSwitch::new(
        PendingBranchTarget::Local,
        7,
    )));
    assert_eq!(
        source_control_scope_nonce(SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        }),
        None
    );
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(Some(PendingRepoSwitch::switch("repo-b", 7)));
    assert_eq!(
        source_control_scope_nonce(SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        }),
        None
    );
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    assert_eq!(
        source_control_scope_nonce(SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        }),
        Some(7)
    );
}

#[test]
fn source_control_scope_rejects_remote_branches() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (current_repo_id, _) = signal(Some("repo-a".to_string()));
    let (active_branch, _) = signal(Some(deve_core::models::PeerId::new("peer-a")));
    let (current_scope_nonce, _) = signal(9u64);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);

    assert_eq!(
        source_control_scope_nonce(SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        }),
        None
    );
}

#[test]
fn source_control_read_scope_allows_remote_branches() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (current_repo_id, _) = signal(Some("repo-a".to_string()));
    let (active_branch, _) = signal(Some(deve_core::models::PeerId::new("peer-a")));
    let (current_scope_nonce, _) = signal(11u64);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);

    assert_eq!(
        source_control_read_scope_nonce(SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        }),
        Some(11)
    );
}
