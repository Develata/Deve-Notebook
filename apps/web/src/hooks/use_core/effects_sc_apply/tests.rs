use super::{
    FsRefreshSignals, apply_doc_diff, refresh_after_fs_change, source_control_refresh_allowed,
};
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::write_gate::RepoWriteGateState;
use crate::hooks::use_core::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch};
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, signal};
use std::cell::Cell;

fn gate_state(
    connection_status: ConnectionStatus,
    handshake_ready: bool,
    writer_ready: bool,
) -> RepoWriteGateState<'static> {
    RepoWriteGateState {
        connection_status,
        load_state: "ready",
        is_read_only: false,
        node_role_probe_failed: false,
        node_role_readable: true,
        handshake_ready,
        writer_ready,
        has_repo: true,
        pending_branch_switch: false,
        pending_repo_switch: false,
    }
}

#[test]
fn apply_doc_diff_preserves_doc_identity() {
    let (diff, set_diff) = signal(None);
    let doc_id = DocId::new();

    apply_doc_diff(Some(doc_id), "notes/a.md", "old", "new", set_diff);

    let session = diff.get_untracked().expect("diff session");
    assert_eq!(session.doc_id, Some(doc_id));
    assert_eq!(session.path, "notes/a.md");
}

#[test]
fn commit_refresh_requires_current_scope_nonce() {
    assert!(!source_control_refresh_allowed(
        7,
        8,
        gate_state(ConnectionStatus::Connected, true, true),
    ));
}

#[test]
fn commit_refresh_blocks_native_recovery_state() {
    assert!(!source_control_refresh_allowed(
        7,
        7,
        gate_state(ConnectionStatus::NativeReprobeRequired, true, true),
    ));
}

#[test]
fn commit_refresh_requires_writer_ready() {
    assert!(!source_control_refresh_allowed(
        7,
        7,
        gate_state(ConnectionStatus::Connected, true, false),
    ));
}

#[test]
fn commit_refresh_allows_ready_local_scope() {
    assert!(source_control_refresh_allowed(
        7,
        7,
        gate_state(ConnectionStatus::Connected, true, true),
    ));
}

fn run_fs_refresh(
    status: ConnectionStatus,
    writer_ready: bool,
) -> (bool, Option<String>, Option<String>, Vec<ClientMessage>) {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(status);
    ws.set_node_role_for_test("main");
    if writer_ready {
        ws.mark_writer_ready("repo-a", 7, "web-light-peer");
    }
    let (current_scope_nonce, _set_current_scope_nonce) = signal(7u64);
    let (current_repo_id, _set_current_repo_id) = signal(Some("repo-a".to_string()));
    let (load_state, _set_load_state) = signal(LoadPhase::Ready);
    let (is_spectator, _set_is_spectator) = signal(false);
    let (handshake_ready, _set_handshake_ready) = signal(true);
    let (pending_branch_switch, _set_pending_branch_switch) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
    let (degraded_sync_mode, _set_degraded_sync_mode) = signal(None);
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
    let (tree_request_id, set_tree_request_id) = signal(None::<String>);
    let scheduled = Cell::new(false);
    let schedule_refresh = || scheduled.set(true);

    refresh_after_fs_change(
        "notes/a.md",
        "modified",
        false,
        FsRefreshSignals {
            expected_scope_nonce: 7,
            current_scope_nonce,
            current_repo_id,
            load_state,
            is_spectator: is_spectator.into(),
            handshake_ready,
            pending_branch_switch,
            pending_repo_switch,
            degraded_sync_mode,
            sync_banner,
            set_sync_banner,
            set_doc_list_request_id,
            set_tree_request_id,
        },
        &schedule_refresh,
        &ws,
    );

    (
        scheduled.get(),
        doc_list_request_id.get_untracked(),
        tree_request_id.get_untracked(),
        ws.drain_sent_for_test(),
    )
}

#[test]
fn fs_refresh_blocks_native_recovery_state() {
    let (scheduled, doc_list_request_id, tree_request_id, sent) =
        run_fs_refresh(ConnectionStatus::NativeReprobeRequired, true);

    assert!(!scheduled);
    assert_eq!(doc_list_request_id, None);
    assert_eq!(tree_request_id, None);
    assert!(sent.is_empty());
}

#[test]
fn fs_refresh_sends_doc_list_when_read_gate_is_ready() {
    let (scheduled, doc_list_request_id, tree_request_id, sent) =
        run_fs_refresh(ConnectionStatus::Connected, true);

    assert!(scheduled);
    let request_id = doc_list_request_id.expect("doc list request");
    assert_eq!(tree_request_id.as_deref(), Some(request_id.as_str()));
    assert_eq!(sent.len(), 1);
    match &sent[0] {
        ClientMessage::ListDocs {
            request_id: sent_request_id,
            scope_nonce,
        } => {
            assert_eq!(sent_request_id, &request_id);
            assert_eq!(*scope_nonce, Some(7));
        }
        other => panic!("expected ListDocs, got {other:?}"),
    }
}
