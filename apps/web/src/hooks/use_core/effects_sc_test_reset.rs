use super::*;
use crate::hooks::use_core::effects_sc_state;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;

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
        display_path: "a.md".into(),
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
    let (notice, set_notice) = signal(Some(SourceControlNotice {
        code: deve_core::protocol::ServerErrorCode::ScNothingToCommit,
        detail: None,
    }));

    clear_repo_scoped_state(effects_sc_state::ScStateResetSignals {
        set_staged,
        set_unstaged,
        set_changes_request_id,
        set_history,
        set_commit_history_request_id: set_history_request_id,
        set_doc_diff_request_id,
        set_diff,
        set_commit_diff_request_id,
        set_commit_diff,
        set_notice,
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
    assert_eq!(notice.get_untracked(), None);
}
