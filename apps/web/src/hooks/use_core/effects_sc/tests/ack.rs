use super::*;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::effects_sc::{ScMessageContext, handle_sc_message};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::storage::DegradedSyncMode;
use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::source_control::ChangeStatus;

#[test]
fn external_apply_ack_only_completes_correlated_transport_request() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let signals = crate::hooks::use_core::state::init_signals(ws.status);
    signals.set_current_repo_id.set(Some(repo_id.to_string()));
    signals.set_current_scope_nonce.set(7);
    let request_id = ws.request_external_apply(7);
    let _ = ws.drain_sent_for_test();
    let schedule_refresh = || {};
    let ctx = ScMessageContext::from_core_signals(signals, &schedule_refresh, &ws);

    assert!(handle_sc_message(
        &ServerMessage::ExternalApplyAck {
            request_id: request_id.clone(),
            receipt: deve_core::source_control::ExternalApplyReceipt {
                repo_id,
                authority_head: deve_core::models::GlobalSeq::from_storage_key(9),
                affected_docs: vec![deve_core::models::DocId::new()],
                applied_target_count: 1,
            },
            repo_id,
            branch: None,
            scope_nonce: deve_core::protocol::ScopeNonce::new(7),
        },
        &ctx,
    ));

    assert!(!ws.complete_external_apply(&request_id));
    assert!(signals.confirmed_changes.get_untracked().is_empty());
}

#[test]
fn commit_ack_dispatch_sets_refresh_request_ids_when_gate_is_ready() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    ws.set_node_role_for_test("main");
    ws.mark_writer_ready(repo_id.to_string(), 7, "web-light-peer");

    let (staged, set_staged) = signal(Vec::<ChangeEntry>::new());
    let (unstaged, set_unstaged) = signal(Vec::<ChangeEntry>::new());
    let (confirmed, set_confirmed) = signal(Vec::<ChangeEntry>::new());
    let (changes_request_id, set_changes_request_id) = signal(None::<String>);
    let (history, set_history) = signal(Vec::<CommitInfo>::new());
    let (history_request_id, set_history_request_id) = signal(None::<String>);
    let (doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
    let (tree_request_id, set_tree_request_id) = signal(None::<String>);
    let (degraded, _set_degraded) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (doc_diff_request_id, set_doc_diff_request_id) = signal(None::<String>);
    let (diff, set_diff) = signal(None::<DiffSessionWire>);
    let (commit_diff_request_id, set_commit_diff_request_id) = signal(None::<String>);
    let (commit_diff, set_commit_diff) = signal(Vec::<CommitFileDiffSummary>::new());
    let (notice, set_notice) = signal(None::<SourceControlNotice>);
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    let (load_state, _) = signal(LoadPhase::Ready);
    let (is_spectator, _) = signal(false);
    let (handshake_ready, _) = signal(true);
    let (active_branch, _) = signal(None::<PeerId>);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    let (current_scope_nonce, _) = signal(7u64);
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

    assert!(handle_sc_message(
        &ServerMessage::CommitAck {
            repo_id: Some(repo_id),
            branch: None,
            scope_nonce: Some(7),
            commit_id: "commit-1".into(),
            timestamp: 1,
        },
        &ctx,
    ));

    assert!(changes_request_id.get_untracked().is_some());
    assert!(history_request_id.get_untracked().is_some());
    assert!(doc_list_request_id.get_untracked().is_some());
    assert_eq!(
        doc_list_request_id.get_untracked(),
        tree_request_id.get_untracked()
    );
    assert_eq!(notice.get_untracked(), None);
    assert!(staged.get_untracked().is_empty());
    assert!(unstaged.get_untracked().is_empty());
    assert!(confirmed.get_untracked().is_empty());
    assert!(history.get_untracked().is_empty());
    assert_eq!(diff.get_untracked(), None);
    assert!(commit_diff.get_untracked().is_empty());

    let sent = ws.drain_sent_for_test();
    assert_eq!(sent.len(), 3);
    match &sent[0] {
        ClientMessage::GetChanges {
            request_id,
            scope_nonce,
        } => {
            assert_eq!(
                Some(request_id),
                changes_request_id.get_untracked().as_ref()
            );
            assert_eq!(*scope_nonce, Some(7));
        }
        other => panic!("expected GetChanges, got {other:?}"),
    }
    match &sent[1] {
        ClientMessage::GetCommitHistory {
            request_id,
            limit,
            scope_nonce,
        } => {
            assert_eq!(
                Some(request_id),
                history_request_id.get_untracked().as_ref()
            );
            assert_eq!(*limit, 50);
            assert_eq!(*scope_nonce, Some(7));
        }
        other => panic!("expected GetCommitHistory, got {other:?}"),
    }
    match &sent[2] {
        ClientMessage::ListDocs {
            request_id,
            scope_nonce,
        } => {
            assert_eq!(
                Some(request_id),
                doc_list_request_id.get_untracked().as_ref()
            );
            assert_eq!(*scope_nonce, Some(7));
        }
        other => panic!("expected ListDocs, got {other:?}"),
    }
}

#[test]
fn commit_ack_clears_open_source_control_diffs() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    ws.set_node_role_for_test("main");
    ws.mark_writer_ready(repo_id.to_string(), 7, "web-light-peer");

    let (_staged, set_staged) = signal(Vec::<ChangeEntry>::new());
    let (_unstaged, set_unstaged) = signal(Vec::<ChangeEntry>::new());
    let (_confirmed, set_confirmed) = signal(Vec::<ChangeEntry>::new());
    let (_changes_request_id, set_changes_request_id) = signal(None::<String>);
    let (_history, set_history) = signal(Vec::<CommitInfo>::new());
    let (history_request_id, set_history_request_id) = signal(None::<String>);
    let (_doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
    let (_tree_request_id, set_tree_request_id) = signal(None::<String>);
    let (degraded, _set_degraded) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (doc_diff_request_id, set_doc_diff_request_id) = signal(None::<String>);
    let (diff, set_diff) = signal(Some(DiffSessionWire::new(
        "external/fs-pending.md".into(),
        String::new(),
        "# External pending".into(),
    )));
    let (commit_diff_request_id, set_commit_diff_request_id) = signal(None::<String>);
    let (commit_diff, set_commit_diff) = signal(vec![test_commit_summary(
        "external/fs-pending.md",
        ChangeStatus::Added,
        None,
    )]);
    let (_notice, set_notice) = signal(None::<SourceControlNotice>);
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    let (load_state, _) = signal(LoadPhase::Ready);
    let (is_spectator, _) = signal(false);
    let (handshake_ready, _) = signal(true);
    let (active_branch, _) = signal(None::<PeerId>);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    let (current_scope_nonce, _) = signal(7u64);
    let schedule_refresh = || {};

    let ctx = ScMessageContext {
        set_staged,
        set_unstaged,
        set_confirmed,
        changes_request_id: _changes_request_id,
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

    assert!(handle_sc_message(
        &ServerMessage::CommitAck {
            repo_id: Some(repo_id),
            branch: None,
            scope_nonce: Some(7),
            commit_id: "commit-1".into(),
            timestamp: 1,
        },
        &ctx,
    ));

    assert!(diff.get_untracked().is_none());
    assert!(commit_diff.get_untracked().is_empty());
}
