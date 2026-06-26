use super::*;

#[test]
fn clears_doc_when_repo_uuid_changes_even_if_name_matches() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(Some("default".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let (pending_repo_switch, set_pending_repo_switch) =
        signal(Some(PendingRepoSwitch::switch("default", 7)));
    let (current_scope_nonce, set_current_scope_nonce) = signal(1u64);
    let (current_doc, set_current_doc) = signal(Some(DocId::new()));
    let next_repo_id = Uuid::new_v4().to_string();

    let outcome = handle_repo_switched(
        "default".to_string(),
        next_repo_id.clone(),
        Some(7),
        RepoSwitchSignals {
            current_repo,
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            current_scope_nonce,
            set_current_scope_nonce,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        },
    );

    assert!(outcome.accepted);
    assert!(outcome.should_refresh);
    assert_eq!(current_repo_id.get_untracked(), Some(next_repo_id));
    assert_eq!(current_doc.get_untracked(), None);
    assert_eq!(pending_repo_switch.get_untracked(), None);
    assert_eq!(current_scope_nonce.get_untracked(), 7);
}

#[test]
fn accepts_repo_switched_after_branch_switch_clears_repo_scope() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(None::<String>);
    let (current_repo_id, set_current_repo_id) = signal(None::<String>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
    let (current_scope_nonce, set_current_scope_nonce) = signal(21u64);
    let (current_doc, set_current_doc) = signal(Some(DocId::new()));
    let repo_id = Uuid::new_v4().to_string();

    let outcome = handle_repo_switched(
        "default".to_string(),
        repo_id.clone(),
        Some(21),
        RepoSwitchSignals {
            current_repo,
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            current_scope_nonce,
            set_current_scope_nonce,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        },
    );

    assert!(outcome.accepted);
    assert!(outcome.should_refresh);
    assert_eq!(current_repo.get_untracked().as_deref(), Some("default"));
    assert_eq!(current_repo_id.get_untracked(), Some(repo_id));
    assert_eq!(current_doc.get_untracked(), None);
    assert_eq!(current_scope_nonce.get_untracked(), 21);
}

#[test]
fn same_repo_rebind_with_new_scope_nonce_requests_refresh() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let repo_id = Uuid::new_v4().to_string();
    let (current_repo, set_current_repo) = signal(Some("default".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(repo_id.clone()));
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
    let (current_scope_nonce, set_current_scope_nonce) = signal(3u64);
    let (current_doc, set_current_doc) = signal(Some(DocId::new()));
    let original_doc = current_doc.get_untracked();

    let outcome = handle_repo_switched(
        "default".to_string(),
        repo_id,
        Some(7),
        RepoSwitchSignals {
            current_repo,
            current_repo_id,
            pending_repo_switch,
            set_pending_repo_switch,
            current_scope_nonce,
            set_current_scope_nonce,
            set_current_repo,
            set_current_repo_id,
            set_current_doc,
        },
    );

    assert!(outcome.accepted);
    assert!(outcome.should_refresh);
    assert_eq!(current_doc.get_untracked(), original_doc);
    assert_eq!(current_scope_nonce.get_untracked(), 7);
}
