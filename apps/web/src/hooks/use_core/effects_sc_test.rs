use super::{
    changes_list_matches_request, clear_repo_scoped_state, commit_diff_matches_request,
    commit_history_matches_request, doc_diff_matches_request, matches_current_repo,
    matches_current_scope,
};
use crate::hooks::use_core::{PendingBranchTarget, diff_session::DiffSessionWire};
use deve_core::models::PeerId;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

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
    let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
    let (pending_repo_switch, _) = signal(None::<String>);
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
    let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
    let (pending_repo_switch, _) = signal(Some("test".to_string()));
    assert!(!matches_current_scope(
        &Some(repo_id),
        &None,
        current_repo_id,
        active_branch,
        pending_branch_switch,
        pending_repo_switch,
    ));
}

#[test]
fn clear_repo_scoped_state_resets_source_control_view() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let entry = ChangeEntry {
        path: "notes/a.md".into(),
        renamed_from: None,
        doc_id: None,
        status: deve_core::source_control::ChangeStatus::Modified,
        has_conflict: false,
    };
    let (staged, set_staged) = signal(vec![entry.clone()]);
    let (unstaged, set_unstaged) = signal(vec![entry]);
    let (changes_request_id, set_changes_request_id) = signal(Some("changes-req".to_string()));
    let (history, set_history) = signal(vec![CommitInfo {
        id: "c1".into(),
        parent_id: None,
        message: "msg".into(),
        timestamp: 1,
        doc_count: 1,
        ledger_seq: 1,
    }]);
    let (history_request_id, set_history_request_id) = signal(Some("history-req".to_string()));
    let (doc_diff_request_id, set_doc_diff_request_id) = signal(Some("doc-req".to_string()));
    let (diff, set_diff) = signal(Some(DiffSessionWire {
        path: "a.md".into(),
        old_content: "old".into(),
        new_content: "new".into(),
        opened_at_ms: 1,
    }));
    let (commit_diff_request_id, set_commit_diff_request_id) =
        signal(Some("commit-req".to_string()));
    let (commit_diff, set_commit_diff) = signal(vec![CommitFileDiff {
        path: "notes/a.md".into(),
        status: deve_core::source_control::ChangeStatus::Modified,
        previous_path: None,
        old_content: "old".into(),
        new_content: "new".into(),
    }]);

    clear_repo_scoped_state(super::super::effects_sc_state::ScStateResetSignals {
        set_staged,
        set_unstaged,
        set_changes_request_id,
        set_history,
        set_commit_history_request_id: set_history_request_id,
        set_doc_diff_request_id,
        set_diff,
        set_commit_diff_request_id,
        set_commit_diff,
    });

    assert!(staged.get_untracked().is_empty());
    assert!(unstaged.get_untracked().is_empty());
    assert_eq!(changes_request_id.get_untracked(), None);
    assert!(history.get_untracked().is_empty());
    assert_eq!(history_request_id.get_untracked(), None);
    assert_eq!(doc_diff_request_id.get_untracked(), None);
    assert_eq!(diff.get_untracked(), None);
    assert_eq!(commit_diff_request_id.get_untracked(), None);
    assert!(commit_diff.get_untracked().is_empty());
}

#[test]
fn doc_diff_accepts_matching_request_or_system_diff() {
    assert!(doc_diff_matches_request(
        &Some("req-1".into()),
        Some("req-1".into()),
    ));
    assert!(!doc_diff_matches_request(
        &Some("stale".into()),
        Some("req-1".into()),
    ));
    assert!(doc_diff_matches_request(&None, None));
    assert!(!doc_diff_matches_request(&None, Some("req-1".into())));
}

#[test]
fn commit_diff_requires_matching_request_id() {
    assert!(commit_diff_matches_request(
        &Some("req-1".into()),
        Some("req-1".into()),
    ));
    assert!(!commit_diff_matches_request(
        &Some("stale".into()),
        Some("req-1".into()),
    ));
    assert!(!commit_diff_matches_request(&None, Some("req-1".into())));
}

#[test]
fn changes_and_history_require_matching_request_id() {
    assert!(changes_list_matches_request(
        &Some("req-1".into()),
        Some("req-1".into()),
    ));
    assert!(!changes_list_matches_request(
        &Some("stale".into()),
        Some("req-1".into()),
    ));
    assert!(changes_list_matches_request(&None, None));
    assert!(!changes_list_matches_request(&None, Some("req-1".into())));

    assert!(commit_history_matches_request(
        &Some("req-1".into()),
        Some("req-1".into()),
    ));
    assert!(!commit_history_matches_request(
        &Some("stale".into()),
        Some("req-1".into()),
    ));
    assert!(!commit_history_matches_request(&None, Some("req-1".into()),));
}
