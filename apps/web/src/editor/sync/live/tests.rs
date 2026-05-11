use super::super::context::SyncContext;
use super::apply_live_op;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::navigation::{NavigationTarget, PendingNavigation};
use crate::hooks::use_core::pending::{
    PendingLocalEditInput, pending_count_for_doc, push_pending_edit,
};
use deve_core::models::{DocId, Op, RepoId};
use deve_core::protocol::{ClientOrigin, ConfirmedOp};
use leptos::prelude::{Callback, GetUntracked, signal};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

fn echoed_live_ctx<'a>(
    ws: &'a WsService,
    repo_id: RepoId,
    doc_id: DocId,
    scope_nonce: u64,
    client_id: u64,
    client_op_id: u64,
    local_version_value: u64,
) -> (
    SyncContext<'a>,
    leptos::prelude::ReadSignal<Option<PendingNavigation>>,
) {
    let (active_branch, _) = signal(None);
    let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    let (current_scope_nonce, _) = signal(scope_nonce);
    let (pending_repo_switch, _) = signal(None::<String>);
    let (handshake_scope_nonce, _) = signal(Some(scope_nonce));
    let (load_state, set_load_state) = signal("ready".to_string());
    let (is_spectator, _) = signal(false);
    let (handshake_ready, _) = signal(true);
    let (open_request_id, _) = signal(0u64);
    let (_, set_content) = signal(String::new());
    let (pending_local_edits, set_pending_local_edits) = signal({
        let mut pending = HashMap::new();
        push_pending_edit(
            &mut pending,
            PendingLocalEditInput {
                repo_id,
                doc_id,
                scope_nonce,
                client_id,
                client_op_id,
                base_version: 0,
                op: Op::Insert {
                    pos: 0,
                    content: "pending".into(),
                },
            },
        );
        pending
    });
    let (pending_navigation, set_pending_navigation) = signal(Some(PendingNavigation {
        target: NavigationTarget::Doc,
        action: Callback::new(|_| {}),
    }));
    let (local_version, set_local_version) = signal(local_version_value);
    let (history, set_history) = signal(Vec::new());
    let (is_playback, _) = signal(false);
    let (_, set_playback_version) = signal(0u64);
    let (_, set_load_progress) = signal((0usize, 0usize));
    let (_, set_load_eta_ms) = signal(0u64);
    let (repo_key, set_repo_key) = signal(None);

    (
        SyncContext {
            doc_id,
            client_id: Some(client_id),
            session_generation: Arc::new(AtomicU64::new(1)),
            ready_generation: Arc::new(AtomicU64::new(1)),
            buffered_live_ops: Arc::new(Mutex::new(Vec::new())),
            buffered_encrypted_ops: Arc::new(Mutex::new(Vec::new())),
            active_branch,
            pending_branch_switch,
            current_repo_id,
            current_scope_nonce,
            pending_repo_switch,
            handshake_scope_nonce,
            load_state,
            is_spectator: is_spectator.into(),
            handshake_ready,
            open_request_id,
            ws,
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
            on_stats: None,
            repo_key,
            set_repo_key,
        },
        pending_navigation,
    )
}

#[test]
fn echoed_new_op_clears_matching_pending_overlay() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::from_u128(7);
    let (ctx, pending_navigation) = echoed_live_ctx(&ws, repo_id, doc_id, 17, 11, 13, 0);

    apply_live_op(
        &ctx,
        ConfirmedOp::new(
            1,
            Op::Insert {
                pos: 0,
                content: "pending".into(),
            },
            Some(ClientOrigin {
                client_id: 11,
                client_op_id: 13,
            }),
        ),
    );

    assert_eq!(
        pending_count_for_doc(&ctx.pending_local_edits.get_untracked(), doc_id),
        0
    );
    assert!(pending_navigation.get_untracked().is_none());
    assert_eq!(ctx.local_version.get_untracked(), 1);
    assert_eq!(ctx.history.get_untracked().len(), 1);
}

#[test]
fn stale_echoed_new_op_clears_pending_without_advancing_history() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::from_u128(7);
    let (ctx, pending_navigation) = echoed_live_ctx(&ws, repo_id, doc_id, 17, 11, 13, 5);

    apply_live_op(
        &ctx,
        ConfirmedOp::new(
            3,
            Op::Insert {
                pos: 0,
                content: "pending".into(),
            },
            Some(ClientOrigin {
                client_id: 11,
                client_op_id: 13,
            }),
        ),
    );

    assert_eq!(
        pending_count_for_doc(&ctx.pending_local_edits.get_untracked(), doc_id),
        0
    );
    assert!(pending_navigation.get_untracked().is_none());
    assert_eq!(ctx.local_version.get_untracked(), 5);
    assert!(ctx.history.get_untracked().is_empty());
}
