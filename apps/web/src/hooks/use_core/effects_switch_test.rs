use super::{BranchSwitchSignals, handle_branch_switched, handle_repo_switched};
use crate::hooks::use_core::{PendingBranchTarget, RepoSwitchSignals};
use deve_core::models::{DocId, PeerId};
use leptos::prelude::*;
use uuid::Uuid;

#[test]
fn clears_doc_when_repo_uuid_changes_even_if_name_matches() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(Some("default".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let (pending_repo_switch, set_pending_repo_switch) = signal(Some("default".to_string()));
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(Some(7));
    let (current_scope_nonce, set_current_scope_nonce) = signal(1u64);
    let (current_doc, set_current_doc) = signal(Some(DocId::new()));
    let next_repo_id = Uuid::new_v4().to_string();

    let changed = handle_repo_switched(
        "default".to_string(),
        next_repo_id.clone(),
        Some(7),
        RepoSwitchSignals {
            current_repo,
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            pending_repo_switch_nonce,
            set_pending_repo_switch_nonce,
            current_scope_nonce,
            set_current_scope_nonce,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        },
    );

    assert!(changed);
    assert_eq!(current_repo_id.get_untracked(), Some(next_repo_id));
    assert_eq!(current_doc.get_untracked(), None);
    assert_eq!(pending_repo_switch.get_untracked(), None);
    assert_eq!(pending_repo_switch_nonce.get_untracked(), None);
    assert_eq!(current_scope_nonce.get_untracked(), 7);
}

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
            active_branch,
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
            active_branch,
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
            active_branch,
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
fn ignores_stale_repo_switched_while_newer_target_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(Some("test".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let (pending_repo_switch, set_pending_repo_switch) = signal(Some("default".to_string()));
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(Some(42));
    let (current_scope_nonce, set_current_scope_nonce) = signal(3u64);
    let (_current_doc, set_current_doc) = signal(Some(DocId::new()));

    let changed = handle_repo_switched(
        "stale".to_string(),
        Uuid::new_v4().to_string(),
        Some(5),
        RepoSwitchSignals {
            current_repo,
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            pending_repo_switch_nonce,
            set_pending_repo_switch_nonce,
            current_scope_nonce,
            set_current_scope_nonce,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        },
    );

    assert!(!changed);
    assert_eq!(
        pending_repo_switch.get_untracked(),
        Some("default".to_string())
    );
}

#[test]
fn ignores_repo_switched_when_nonce_is_stale_for_same_target() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(Some("test".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let (pending_repo_switch, set_pending_repo_switch) = signal(Some("default".to_string()));
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(Some(42));
    let (current_scope_nonce, set_current_scope_nonce) = signal(3u64);
    let (current_doc, set_current_doc) = signal(Some(DocId::new()));
    let current_doc_id = current_doc.get_untracked();

    let changed = handle_repo_switched(
        "default".to_string(),
        Uuid::new_v4().to_string(),
        Some(5),
        RepoSwitchSignals {
            current_repo,
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            pending_repo_switch_nonce,
            set_pending_repo_switch_nonce,
            current_scope_nonce,
            set_current_scope_nonce,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        },
    );

    assert!(!changed);
    assert_eq!(
        pending_repo_switch.get_untracked(),
        Some("default".to_string())
    );
    assert_eq!(current_doc.get_untracked(), current_doc_id);
}

#[test]
fn ignores_repo_switched_without_pending_when_repo_differs() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(Some("default".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let (pending_repo_switch, set_pending_repo_switch) = signal(None);
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(None);
    let (current_scope_nonce, set_current_scope_nonce) = signal(5u64);
    let doc_id = DocId::new();
    let (current_doc, set_current_doc) = signal(Some(doc_id));

    assert!(!handle_repo_switched(
        "test".to_string(),
        Uuid::new_v4().to_string(),
        Some(99),
        RepoSwitchSignals {
            current_repo,
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            pending_repo_switch_nonce,
            set_pending_repo_switch_nonce,
            current_scope_nonce,
            set_current_scope_nonce,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        },
    ));
    assert_eq!(current_doc.get_untracked(), Some(doc_id));
}

#[test]
fn accepts_repo_switched_after_branch_switch_clears_repo_scope() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(None::<String>);
    let (current_repo_id, set_current_repo_id) = signal(None::<String>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<String>);
    let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(Some(21));
    let (current_scope_nonce, set_current_scope_nonce) = signal(8u64);
    let (current_doc, set_current_doc) = signal(Some(DocId::new()));
    let repo_id = Uuid::new_v4().to_string();

    let changed = handle_repo_switched(
        "default".to_string(),
        repo_id.clone(),
        Some(21),
        RepoSwitchSignals {
            current_repo,
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            pending_repo_switch_nonce,
            set_pending_repo_switch_nonce,
            current_scope_nonce,
            set_current_scope_nonce,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        },
    );

    assert!(changed);
    assert_eq!(current_repo.get_untracked().as_deref(), Some("default"));
    assert_eq!(current_repo_id.get_untracked(), Some(repo_id));
    assert_eq!(current_doc.get_untracked(), None);
    assert_eq!(pending_repo_switch_nonce.get_untracked(), None);
    assert_eq!(current_scope_nonce.get_untracked(), 21);
}
