use super::*;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::navigation::PendingNavigation;
use crate::runtime::document::pending::{
    PendingLocalEditInput, pending_count_for_doc, push_pending_edit,
};
use crate::runtime::domain::{PendingBranchSwitch, PendingRepoSwitch};
use crate::runtime::projection_recovery::ProjectionRecoveryCoordinator;
use deve_core::models::{DocId, Op, RepoId};
use deve_core::protocol::{
    ClientMessage, ConfirmedOp, ProjectionRecoveryCause, ProjectionRecoveryPlan,
    ProjectionRecoveryRequired,
};
use leptos::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[test]
fn projection_recovery_reopens_exact_document_without_dropping_pending_overlay() {
    let owner = leptos::reactive::owner::Owner::new();
    owner.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::new();
    let (active_branch, _) = signal(None);
    let (current_doc, _) = signal(Some(doc_id));
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    let (current_scope_nonce, _) = signal(7u64);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    let (handshake_scope_nonce, _) = signal(Some(7u64));
    let (load_state, set_load_state) = signal(LoadPhase::Ready);
    let (is_spectator, _) = signal(false);
    let (handshake_ready, _) = signal(true);
    let (open_request_id, set_open_request_id) = signal(0u64);
    let (content, set_content) = signal("confirmed".to_string());
    let (pending_local_edits, set_pending_local_edits) = signal({
        let mut pending = HashMap::new();
        push_pending_edit(
            &mut pending,
            PendingLocalEditInput {
                repo_id,
                doc_id,
                scope_nonce: 7,
                client_id: 11,
                client_op_id: 13,
                base_version: 0,
                op: Op::Insert {
                    pos: 0,
                    content: "pending".into(),
                },
            },
        );
        pending
    });
    let (_, set_pending_navigation) = signal(None::<PendingNavigation>);
    let (local_version, set_local_version) = signal(3u64);
    let (history, set_history) = signal(Vec::new());
    let (is_playback, _) = signal(false);
    let (playback_version, set_playback_version) = signal(3u64);
    let (_, set_load_progress) = signal((0usize, 0usize));
    let (_, set_load_eta_ms) = signal(0u64);
    let (_, set_editor_sync_failure) = signal(None);
    let (snapshot_reopen_attempted, set_snapshot_reopen_attempted) = signal(false);
    let (repo_key, set_repo_key) = signal(None);
    let session_generation = Arc::new(AtomicU64::new(1));
    let ready_generation = Arc::new(AtomicU64::new(1));
    let buffered_live_ops = Arc::new(Mutex::new(vec![ConfirmedOp::new(
        4,
        Op::Insert {
            pos: 0,
            content: "stale".into(),
        },
        None,
    )]));
    let buffered_encrypted_ops = Arc::new(Mutex::new(Vec::new()));
    let projection_recovery = ProjectionRecoveryCoordinator::default();
    let ctx = SyncContext {
        doc_id,
        client_id: Some(11),
        session_generation: session_generation.clone(),
        ready_generation: ready_generation.clone(),
        pending_resend_generation: Arc::new(AtomicU64::new(0)),
        projection_recovery,
        buffered_live_ops: buffered_live_ops.clone(),
        buffered_encrypted_ops,
        active_branch,
        current_doc,
        pending_branch_switch,
        current_repo_id,
        current_scope_nonce,
        pending_repo_switch,
        handshake_scope_nonce,
        load_state,
        is_spectator: is_spectator.into(),
        handshake_ready,
        open_request_id,
        set_open_request_id,
        ws: &ws,
        content,
        set_content,
        pending_local_edits,
        set_pending_local_edits,
        set_pending_navigation,
        local_version,
        set_local_version,
        history,
        set_history,
        is_playback,
        set_playback_version,
        set_load_state,
        set_load_progress,
        set_load_eta_ms,
        set_editor_sync_failure,
        snapshot_reopen_attempted,
        set_snapshot_reopen_attempted,
        on_stats: None,
        repo_key,
        set_repo_key,
    };

    handle_required(
        &ctx,
        ProjectionRecoveryRequired {
            repo_id,
            branch: None,
            scope_nonce: Some(7),
            cause: ProjectionRecoveryCause::ExternalApply,
            plan: ProjectionRecoveryPlan::external_apply(vec![doc_id]),
        },
    );

    assert_eq!(load_state.get_untracked(), LoadPhase::Resyncing);
    assert_eq!(open_request_id.get_untracked(), 2);
    assert_eq!(local_version.get_untracked(), 0);
    assert_eq!(playback_version.get_untracked(), 0);
    assert!(history.get_untracked().is_empty());
    assert_eq!(ready_generation.load(Ordering::Relaxed), 0);
    assert!(buffered_live_ops.lock().unwrap().is_empty());
    assert_eq!(
        pending_count_for_doc(&pending_local_edits.get_untracked(), doc_id),
        1
    );
    let sent = ws.drain_sent_for_test();
    assert!(matches!(
        sent.as_slice(),
        [ClientMessage::OpenDoc {
            doc_id: actual_doc,
            request_id: 2,
            scope_nonce: Some(7),
        }] if *actual_doc == doc_id
    ));
}
