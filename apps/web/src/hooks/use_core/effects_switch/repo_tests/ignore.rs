use super::*;

#[test]
fn ignores_stale_repo_switched_while_newer_target_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(Some("test".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let pending_target = Uuid::new_v4();
    let (pending_repo_switch, set_pending_repo_switch) = signal(Some(PendingRepoSwitch::switch(
        "default",
        pending_target,
        42,
    )));
    let (current_scope_nonce, set_current_scope_nonce) = signal(3u64);
    let (_current_doc, set_current_doc) = signal(Some(DocId::new()));

    let outcome = handle_repo_switched(
        "stale".to_string(),
        Uuid::new_v4().to_string(),
        Some(5),
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

    assert!(!outcome.accepted);
    assert!(!outcome.should_refresh);
    assert_eq!(
        pending_repo_switch
            .get_untracked()
            .map(|pending| pending.expected_name),
        Some("default".to_string())
    );
}

#[test]
fn ignores_repo_switched_when_nonce_is_stale_for_same_target() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(Some("test".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let pending_target = Uuid::new_v4();
    let (pending_repo_switch, set_pending_repo_switch) = signal(Some(PendingRepoSwitch::switch(
        "default",
        pending_target,
        42,
    )));
    let (current_scope_nonce, set_current_scope_nonce) = signal(3u64);
    let (current_doc, set_current_doc) = signal(Some(DocId::new()));
    let current_doc_id = current_doc.get_untracked();

    let outcome = handle_repo_switched(
        "default".to_string(),
        pending_target.to_string(),
        Some(5),
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

    assert!(!outcome.accepted);
    assert!(!outcome.should_refresh);
    assert_eq!(
        pending_repo_switch
            .get_untracked()
            .map(|pending| pending.expected_name),
        Some("default".to_string())
    );
    assert_eq!(current_doc.get_untracked(), current_doc_id);
}

#[test]
fn duplicate_alias_cannot_replace_exact_pending_repo_id() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let pending_target = Uuid::new_v4();
    let (current_repo, set_current_repo) = signal(Some("test".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let (pending_repo_switch, set_pending_repo_switch) = signal(Some(PendingRepoSwitch::switch(
        "duplicate",
        pending_target,
        42,
    )));
    let (current_scope_nonce, set_current_scope_nonce) = signal(3u64);
    let (current_doc, set_current_doc) = signal(Some(DocId::new()));
    let original_doc = current_doc.get_untracked();

    let outcome = handle_repo_switched(
        "duplicate".to_string(),
        Uuid::new_v4().to_string(),
        Some(42),
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

    assert!(!outcome.accepted);
    assert_eq!(current_doc.get_untracked(), original_doc);
    assert_eq!(
        pending_repo_switch
            .get_untracked()
            .and_then(|pending| pending.expected_repo_id),
        Some(pending_target)
    );
}

#[test]
fn ignores_repo_switched_without_pending_when_repo_differs() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();

    let (current_repo, set_current_repo) = signal(Some("default".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
    let (current_scope_nonce, set_current_scope_nonce) = signal(5u64);
    let doc_id = DocId::new();
    let (current_doc, set_current_doc) = signal(Some(doc_id));

    let outcome = handle_repo_switched(
        "test".to_string(),
        Uuid::new_v4().to_string(),
        Some(99),
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

    assert!(!outcome.accepted);
    assert!(!outcome.should_refresh);
    assert_eq!(current_doc.get_untracked(), Some(doc_id));
}
