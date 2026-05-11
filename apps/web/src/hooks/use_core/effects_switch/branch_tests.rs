use super::{BranchSwitchSignals, handle_branch_switched};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;
use leptos::prelude::*;

#[test]
fn branch_switch_reports_when_scope_changed() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (active_branch, set_active_branch) = signal(Some(PeerId::new("peer-a")));
    let (pending_branch_switch, set_pending_branch_switch) =
        signal(Some(PendingBranchTarget::Shadow("peer-b".into())));
    let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(Some(11));
    let changed = handle_branch_switched(
        Some("peer-b".into()),
        true,
        Some(11),
        BranchSwitchSignals {
            pending_branch_switch,
            pending_branch_switch_nonce,
            set_pending_branch_switch,
            set_pending_branch_switch_nonce,
            set_active_branch,
        },
    );

    assert!(changed);
    assert_eq!(active_branch.get_untracked(), Some(PeerId::new("peer-b")));
    assert_eq!(pending_branch_switch.get_untracked(), None);
    assert_eq!(pending_branch_switch_nonce.get_untracked(), None);
}

#[test]
fn ignores_branch_switched_without_pending_target() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (active_branch, set_active_branch) = signal(Some(PeerId::new("peer-a")));
    let (pending_branch_switch, set_pending_branch_switch) = signal(None);
    let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(None);

    assert!(!handle_branch_switched(
        Some("peer-b".into()),
        true,
        Some(3),
        BranchSwitchSignals {
            pending_branch_switch,
            pending_branch_switch_nonce,
            set_pending_branch_switch,
            set_pending_branch_switch_nonce,
            set_active_branch,
        },
    ));
    assert_eq!(active_branch.get_untracked(), Some(PeerId::new("peer-a")));
}

#[test]
fn ignores_branch_switched_with_stale_nonce() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (active_branch, set_active_branch) = signal(Some(PeerId::new("peer-a")));
    let (pending_branch_switch, set_pending_branch_switch) =
        signal(Some(PendingBranchTarget::Shadow("peer-b".into())));
    let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(Some(9));

    assert!(!handle_branch_switched(
        Some("peer-b".into()),
        true,
        Some(7),
        BranchSwitchSignals {
            pending_branch_switch,
            pending_branch_switch_nonce,
            set_pending_branch_switch,
            set_pending_branch_switch_nonce,
            set_active_branch,
        },
    ));
    assert_eq!(active_branch.get_untracked(), Some(PeerId::new("peer-a")));
}

#[test]
fn accepts_same_branch_rebind_with_new_scope_nonce() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (active_branch, set_active_branch) = signal(Some(PeerId::new("peer-a")));
    let (pending_branch_switch, set_pending_branch_switch) =
        signal(Some(PendingBranchTarget::Shadow("peer-a".into())));
    let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(Some(13));

    assert!(handle_branch_switched(
        Some("peer-a".into()),
        true,
        Some(13),
        BranchSwitchSignals {
            pending_branch_switch,
            pending_branch_switch_nonce,
            set_pending_branch_switch,
            set_pending_branch_switch_nonce,
            set_active_branch,
        },
    ));
    assert_eq!(active_branch.get_untracked(), Some(PeerId::new("peer-a")));
    assert_eq!(pending_branch_switch.get_untracked(), None);
    assert_eq!(pending_branch_switch_nonce.get_untracked(), None);
}
