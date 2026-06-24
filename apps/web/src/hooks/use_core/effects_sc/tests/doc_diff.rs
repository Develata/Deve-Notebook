use super::*;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::effects_sc::{ScMessageContext, handle_sc_message};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::storage::DegradedSyncMode;
use deve_core::models::DocId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};

struct DocDiffDispatchResult {
    handled: bool,
    request_id: Option<String>,
    diff: Option<DiffSessionWire>,
    notice: Option<SourceControlNotice>,
}

fn dispatch_doc_diff(
    current_repo_id_value: uuid::Uuid,
    active_branch_value: Option<PeerId>,
    message_branch: Option<PeerId>,
    message_scope_nonce: Option<u64>,
) -> DocDiffDispatchResult {
    dispatch_doc_diff_from_repo(
        current_repo_id_value,
        current_repo_id_value,
        active_branch_value,
        message_branch,
        message_scope_nonce,
    )
}

fn dispatch_doc_diff_from_repo(
    current_repo_id_value: uuid::Uuid,
    message_repo_id_value: uuid::Uuid,
    active_branch_value: Option<PeerId>,
    message_branch: Option<PeerId>,
    message_scope_nonce: Option<u64>,
) -> DocDiffDispatchResult {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (staged, set_staged) = signal(Vec::<ChangeEntry>::new());
    let (unstaged, set_unstaged) = signal(Vec::<ChangeEntry>::new());
    let (confirmed, set_confirmed) = signal(Vec::<ChangeEntry>::new());
    let (changes_request_id, set_changes_request_id) = signal(None::<String>);
    let (history, set_history) = signal(Vec::<CommitInfo>::new());
    let (history_request_id, set_history_request_id) = signal(None::<String>);
    let (_doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
    let (_tree_request_id, set_tree_request_id) = signal(None::<String>);
    let (degraded, _set_degraded) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (doc_diff_request_id, set_doc_diff_request_id) = signal(Some("doc-req-1".to_string()));
    let (diff, set_diff) = signal(Some(DiffSessionWire {
        doc_id: None,
        path: "stale.md".into(),
        display_path: "stale.md".into(),
        old_content: "stale-old".into(),
        new_content: "stale-new".into(),
        merge_conflict: None,
        opened_at_ms: 1,
    }));
    let (commit_diff_request_id, set_commit_diff_request_id) = signal(None::<String>);
    let (commit_diff, set_commit_diff) = signal(Vec::<CommitFileDiff>::new());
    let (notice, set_notice) = signal(Some(SourceControlNotice {
        code: ServerErrorCode::ScDocNotFound,
        detail: Some("previous error".into()),
    }));
    let (current_repo_id, _) = signal(Some(current_repo_id_value.to_string()));
    let (load_state, _) = signal("ready".to_string());
    let (is_spectator, _) = signal(active_branch_value.is_some());
    let (handshake_ready, _) = signal(active_branch_value.is_none());
    let (active_branch, _) = signal(active_branch_value);
    let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
    let (pending_repo_switch, _) = signal(None::<String>);
    let (current_scope_nonce, _) = signal(17u64);
    let schedule_refresh = || {};
    let doc_id = DocId::new();

    let ctx = ScMessageContext {
        set_staged,
        set_unstaged,
        set_confirmed,
        changes_request_id,
        set_changes_request_id,
        set_history,
        commit_history_request_id: history_request_id,
        set_commit_history_request_id: set_history_request_id,
        set_doc_list_request_id,
        set_tree_request_id,
        degraded_sync_mode: degraded,
        sync_banner,
        set_sync_banner,
        doc_diff_request_id,
        set_doc_diff_request_id,
        diff,
        set_diff,
        commit_diff_request_id,
        set_commit_diff_request_id,
        set_commit_diff,
        set_notice,
        current_repo_id,
        load_state,
        is_spectator: is_spectator.into(),
        handshake_ready,
        active_branch,
        pending_branch_switch,
        pending_repo_switch,
        current_scope_nonce,
        schedule_refresh: &schedule_refresh,
        ws: &ws,
    };
    let handled = handle_sc_message(
        &ServerMessage::DocDiff {
            request_id: Some("doc-req-1".into()),
            repo_id: Some(message_repo_id_value),
            branch: message_branch,
            scope_nonce: message_scope_nonce,
            doc_id: Some(doc_id),
            path: "notes/a.md".into(),
            old_content: "old".into(),
            new_content: "new".into(),
        },
        &ctx,
    );

    assert!(staged.get_untracked().is_empty());
    assert!(unstaged.get_untracked().is_empty());
    assert!(confirmed.get_untracked().is_empty());
    assert!(history.get_untracked().is_empty());
    assert!(commit_diff.get_untracked().is_empty());

    DocDiffDispatchResult {
        handled,
        request_id: doc_diff_request_id.get_untracked(),
        diff: diff.get_untracked(),
        notice: notice.get_untracked(),
    }
}

fn assert_stale_doc_diff_preserved(result: DocDiffDispatchResult) {
    assert!(result.handled);
    assert_eq!(result.request_id.as_deref(), Some("doc-req-1"));
    let diff = result.diff.expect("stale diff");
    assert_eq!(diff.path, "stale.md");
    assert_eq!(diff.old_content, "stale-old");
    assert_eq!(diff.new_content, "stale-new");
    assert!(result.notice.is_some());
}

#[test]
fn doc_diff_dispatch_accepts_remote_branch_scope() {
    let repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_doc_diff(repo_id, Some(branch.clone()), Some(branch), Some(17));

    assert!(result.handled);
    assert_eq!(result.request_id, None);
    assert_eq!(result.notice, None);
    let diff = result.diff.expect("doc diff");
    assert_eq!(diff.path, "notes/a.md");
    assert_eq!(diff.old_content, "old");
    assert_eq!(diff.new_content, "new");
    assert!(diff.doc_id.is_some());
}

#[test]
fn doc_diff_dispatch_rejects_stale_scope_nonce() {
    let repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_doc_diff(repo_id, Some(branch.clone()), Some(branch), Some(16));

    assert_stale_doc_diff_preserved(result);
}

#[test]
fn doc_diff_dispatch_rejects_other_branch() {
    let repo_id = uuid::Uuid::new_v4();
    let result = dispatch_doc_diff(
        repo_id,
        Some(PeerId::new("peer-a")),
        Some(PeerId::new("peer-b")),
        Some(17),
    );

    assert_stale_doc_diff_preserved(result);
}

#[test]
fn doc_diff_dispatch_rejects_other_repo() {
    let current_repo_id = uuid::Uuid::new_v4();
    let message_repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_doc_diff_from_repo(
        current_repo_id,
        message_repo_id,
        Some(branch.clone()),
        Some(branch),
        Some(17),
    );

    assert_stale_doc_diff_preserved(result);
}
