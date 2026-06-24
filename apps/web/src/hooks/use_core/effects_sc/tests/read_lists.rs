use super::*;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::effects_sc::{ScMessageContext, handle_sc_message};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::storage::DegradedSyncMode;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::source_control::{ChangeDomain, ChangeStatus};

#[derive(Clone, Copy)]
enum ReadListKind {
    Changes,
    History,
}

struct ReadListDispatchResult {
    handled: bool,
    changes_request_id: Option<String>,
    staged: Vec<ChangeEntry>,
    unstaged: Vec<ChangeEntry>,
    confirmed: Vec<ChangeEntry>,
    history_request_id: Option<String>,
    history: Vec<CommitInfo>,
    notice: Option<SourceControlNotice>,
}

fn change(path: &str, status: ChangeStatus) -> ChangeEntry {
    change_in_domain(path, status, ChangeDomain::WorkingDirectory)
}

fn change_in_domain(path: &str, status: ChangeStatus, domain: ChangeDomain) -> ChangeEntry {
    ChangeEntry {
        path: path.into(),
        renamed_from: None,
        doc_id: None,
        status,
        has_conflict: false,
        domain,
        base_seq: None,
        target_seq: None,
    }
}

fn commit(id: &str, message: &str) -> CommitInfo {
    CommitInfo {
        id: id.into(),
        parent_id: None,
        message: message.into(),
        timestamp: 1,
        doc_count: 1,
        ledger_seq: 1,
    }
}

fn dispatch_read_list(
    kind: ReadListKind,
    current_repo_id_value: uuid::Uuid,
    active_branch_value: Option<PeerId>,
    message_branch: Option<PeerId>,
    message_scope_nonce: Option<u64>,
) -> ReadListDispatchResult {
    dispatch_read_list_from_repo(
        kind,
        current_repo_id_value,
        current_repo_id_value,
        active_branch_value,
        message_branch,
        message_scope_nonce,
    )
}

fn dispatch_read_list_from_repo(
    kind: ReadListKind,
    current_repo_id_value: uuid::Uuid,
    message_repo_id_value: uuid::Uuid,
    active_branch_value: Option<PeerId>,
    message_branch: Option<PeerId>,
    message_scope_nonce: Option<u64>,
) -> ReadListDispatchResult {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (staged, set_staged) = signal(vec![change("stale-staged.md", ChangeStatus::Modified)]);
    let (unstaged, set_unstaged) = signal(vec![change("stale-unstaged.md", ChangeStatus::Deleted)]);
    let (confirmed, set_confirmed) = signal(vec![change_in_domain(
        "stale-confirmed.md",
        ChangeStatus::Modified,
        ChangeDomain::ConfirmedLedger,
    )]);
    let (changes_request_id, set_changes_request_id) = signal(Some("changes-req-1".to_string()));
    let (history, set_history) = signal(vec![commit("stale", "stale commit")]);
    let (history_request_id, set_history_request_id) = signal(Some("history-req-1".to_string()));
    let (_doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
    let (_tree_request_id, set_tree_request_id) = signal(None::<String>);
    let (degraded, _set_degraded) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (doc_diff_request_id, set_doc_diff_request_id) = signal(None::<String>);
    let (diff, set_diff) = signal(None::<DiffSessionWire>);
    let (commit_diff_request_id, set_commit_diff_request_id) = signal(None::<String>);
    let (commit_diff, set_commit_diff) = signal(Vec::<CommitFileDiff>::new());
    let (notice, set_notice) = signal(Some(SourceControlNotice {
        code: ServerErrorCode::ScRepoContextInvalid,
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
    let msg = match kind {
        ReadListKind::Changes => ServerMessage::ChangesList {
            request_id: Some("changes-req-1".into()),
            repo_id: Some(message_repo_id_value),
            branch: message_branch,
            scope_nonce: message_scope_nonce,
            staged: vec![change("fresh-staged.md", ChangeStatus::Added)],
            unstaged: vec![change("fresh-unstaged.md", ChangeStatus::Modified)],
            confirmed: vec![change_in_domain(
                "fresh-confirmed.md",
                ChangeStatus::Modified,
                ChangeDomain::ConfirmedLedger,
            )],
        },
        ReadListKind::History => ServerMessage::CommitHistory {
            request_id: Some("history-req-1".into()),
            repo_id: Some(message_repo_id_value),
            branch: message_branch,
            scope_nonce: message_scope_nonce,
            commits: vec![commit("fresh", "fresh commit")],
        },
    };
    let handled = handle_sc_message(&msg, &ctx);

    assert_eq!(diff.get_untracked(), None);
    assert!(commit_diff.get_untracked().is_empty());

    ReadListDispatchResult {
        handled,
        changes_request_id: changes_request_id.get_untracked(),
        staged: staged.get_untracked(),
        unstaged: unstaged.get_untracked(),
        confirmed: confirmed.get_untracked(),
        history_request_id: history_request_id.get_untracked(),
        history: history.get_untracked(),
        notice: notice.get_untracked(),
    }
}

fn assert_changes_preserved(result: &ReadListDispatchResult) {
    assert_eq!(result.changes_request_id.as_deref(), Some("changes-req-1"));
    assert_eq!(result.staged[0].path, "stale-staged.md");
    assert_eq!(result.unstaged[0].path, "stale-unstaged.md");
    assert_eq!(result.confirmed[0].path, "stale-confirmed.md");
    assert!(result.notice.is_some());
}

fn assert_history_preserved(result: &ReadListDispatchResult) {
    assert_eq!(result.history_request_id.as_deref(), Some("history-req-1"));
    assert_eq!(result.history[0].id, "stale");
    assert!(result.notice.is_some());
}

mod changes;
mod history;
