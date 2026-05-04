use super::*;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::effects_sc::{handle_sc_message, sc_message_context};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::storage::DegradedSyncMode;
use deve_core::models::DocId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::source_control::ChangeStatus;

struct CommitDiffDispatchResult {
    handled: bool,
    request_id: Option<String>,
    diffs: Vec<CommitFileDiff>,
    notice: Option<SourceControlNotice>,
}

fn dispatch_commit_diff(
    current_repo_id_value: uuid::Uuid,
    active_branch_value: Option<PeerId>,
    message_branch: Option<PeerId>,
    message_scope_nonce: Option<u64>,
) -> CommitDiffDispatchResult {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (staged, set_staged) = signal(Vec::<ChangeEntry>::new());
    let (unstaged, set_unstaged) = signal(Vec::<ChangeEntry>::new());
    let (changes_request_id, set_changes_request_id) = signal(None::<String>);
    let (history, set_history) = signal(Vec::<CommitInfo>::new());
    let (history_request_id, set_history_request_id) = signal(None::<String>);
    let (_doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
    let (_tree_request_id, set_tree_request_id) = signal(None::<String>);
    let (degraded, _set_degraded) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (doc_diff_request_id, set_doc_diff_request_id) = signal(None::<String>);
    let (diff, set_diff) = signal(None::<DiffSessionWire>);
    let (commit_diff_request_id, set_commit_diff_request_id) = signal(Some("req-1".to_string()));
    let (commit_diff, set_commit_diff) = signal(Vec::<CommitFileDiff>::new());
    let (notice, set_notice) = signal(Some(SourceControlNotice {
        code: ServerErrorCode::ScCommitDiffUnprojectable,
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

    let ctx = sc_message_context(
        set_staged,
        set_unstaged,
        changes_request_id,
        set_changes_request_id,
        set_history,
        history_request_id,
        set_history_request_id,
        set_doc_list_request_id,
        set_tree_request_id,
        degraded,
        sync_banner,
        set_sync_banner,
        doc_diff_request_id,
        set_doc_diff_request_id,
        set_diff,
        commit_diff_request_id,
        set_commit_diff_request_id,
        set_commit_diff,
        set_notice,
        current_repo_id,
        load_state,
        is_spectator.into(),
        handshake_ready,
        active_branch,
        pending_branch_switch,
        pending_repo_switch,
        current_scope_nonce,
        &schedule_refresh,
        &ws,
    );
    let handled = handle_sc_message(
        &ServerMessage::CommitDiffResult {
            request_id: Some("req-1".into()),
            repo_id: Some(current_repo_id_value),
            branch: message_branch,
            scope_nonce: message_scope_nonce,
            diffs: vec![CommitFileDiff {
                doc_id: Some(DocId::new()),
                path: "renamed.md".into(),
                status: ChangeStatus::Renamed,
                previous_path: Some("note.md".into()),
                old_content: "hello".into(),
                new_content: "hello".into(),
            }],
        },
        &ctx,
    );

    assert!(staged.get_untracked().is_empty());
    assert!(unstaged.get_untracked().is_empty());
    assert!(history.get_untracked().is_empty());
    assert_eq!(diff.get_untracked(), None);

    CommitDiffDispatchResult {
        handled,
        request_id: commit_diff_request_id.get_untracked(),
        diffs: commit_diff.get_untracked(),
        notice: notice.get_untracked(),
    }
}

#[test]
fn commit_diff_dispatch_accepts_remote_branch_scope() {
    let repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_commit_diff(repo_id, Some(branch.clone()), Some(branch), Some(17));

    assert!(result.handled);
    assert_eq!(result.request_id, None);
    assert_eq!(result.notice, None);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].previous_path.as_deref(), Some("note.md"));
    assert_eq!(result.diffs[0].path, "renamed.md");
    assert_eq!(result.diffs[0].status, ChangeStatus::Renamed);
}

#[test]
fn commit_diff_dispatch_rejects_stale_scope_nonce() {
    let repo_id = uuid::Uuid::new_v4();
    let branch = PeerId::new("peer-a");
    let result = dispatch_commit_diff(repo_id, Some(branch.clone()), Some(branch), Some(16));

    assert!(result.handled);
    assert_eq!(result.request_id.as_deref(), Some("req-1"));
    assert!(result.diffs.is_empty());
    assert!(result.notice.is_some());
}

#[test]
fn commit_diff_dispatch_rejects_other_branch() {
    let repo_id = uuid::Uuid::new_v4();
    let result = dispatch_commit_diff(
        repo_id,
        Some(PeerId::new("peer-a")),
        Some(PeerId::new("peer-b")),
        Some(17),
    );

    assert!(result.handled);
    assert_eq!(result.request_id.as_deref(), Some("req-1"));
    assert!(result.diffs.is_empty());
    assert!(result.notice.is_some());
}
