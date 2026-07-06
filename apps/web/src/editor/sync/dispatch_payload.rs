//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 07_network#web-ws-runtime
//!
use super::context::SyncContext;
use super::decrypt;
use super::history_resend;
use super::key::handle_key_provide;
use super::scope::matches_scoped_message;
use deve_core::protocol::ServerError;
use deve_core::security::EncryptedOp;
use leptos::prelude::Set;

pub fn handle_write_ready_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
) {
    if super::accepts_current_sync_payload(ctx, repo_id, branch, scope_nonce) {
        history_resend::resend_pending_edits_if_ready(ctx);
    }
}

pub fn handle_sync_push_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
    ops: &[EncryptedOp],
) {
    if super::accepts_current_sync_payload(ctx, repo_id, branch, scope_nonce) {
        decrypt::handle_sync_push(ctx, ops);
    }
}

pub fn handle_key_provide_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
    repo_key: &[u8],
) {
    if matches_scoped_message(
        super::current_scoped_message_scope(ctx),
        Some(repo_id),
        branch,
        Some(scope_nonce),
    ) {
        handle_key_provide(ctx, repo_key);
    }
}

pub fn handle_key_denied_message(
    ctx: &SyncContext,
    repo_id: Option<deve_core::models::RepoId>,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
    error: &ServerError,
) {
    if matches_scoped_message(
        super::current_scoped_message_scope(ctx),
        repo_id,
        branch,
        Some(scope_nonce),
    ) {
        ctx.set_repo_key.set(None);
        leptos::logging::warn!("KeyDenied: code={:?} detail={:?}", error.code, error.detail);
    }
}

#[cfg(test)]
mod tests {
    use super::super::context::SyncContext;
    use super::handle_write_ready_message;
    use crate::api::{ConnectionStatus, WsService};
    use crate::hooks::use_core::navigation::PendingNavigation;
    use crate::runtime::document::pending::{
        PendingLocalEditInput, pending_count_for_doc, push_pending_edit,
    };
    use crate::runtime::domain::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch};
    use deve_core::models::{DocId, Op, RepoId};
    use deve_core::protocol::ClientMessage;
    use leptos::prelude::{GetUntracked, signal};
    use std::collections::HashMap;
    use std::sync::atomic::AtomicU64;
    use std::sync::{Arc, Mutex};

    fn write_ready_ctx<'a>(
        ws: &'a WsService,
        repo_id: RepoId,
        doc_id: DocId,
        scope_nonce: u64,
    ) -> SyncContext<'a> {
        write_ready_ctx_with_pending_scope(ws, repo_id, doc_id, scope_nonce, scope_nonce)
    }

    fn write_ready_ctx_with_pending_scope<'a>(
        ws: &'a WsService,
        repo_id: RepoId,
        doc_id: DocId,
        scope_nonce: u64,
        pending_scope_nonce: u64,
    ) -> SyncContext<'a> {
        let (active_branch, _) = signal(None);
        let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
        let (current_repo_id, _) = signal(Some(repo_id.to_string()));
        let (current_scope_nonce, _) = signal(scope_nonce);
        let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
        let (handshake_scope_nonce, _) = signal(Some(scope_nonce));
        let (load_state, set_load_state) = signal(LoadPhase::Ready);
        let (is_spectator, _) = signal(false);
        let (handshake_ready, _) = signal(true);
        let (open_request_id, _) = signal(0u64);
        let (content, set_content) = signal(String::new());
        let (pending_local_edits, set_pending_local_edits) = signal({
            let mut pending = HashMap::new();
            push_pending_edit(
                &mut pending,
                PendingLocalEditInput {
                    repo_id,
                    doc_id,
                    scope_nonce: pending_scope_nonce,
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
        let (local_version, set_local_version) = signal(0u64);
        let (history, set_history) = signal(Vec::new());
        let (is_playback, _) = signal(false);
        let (playback_version, set_playback_version) = signal(0u64);
        let (load_progress, set_load_progress) = signal((0usize, 0usize));
        let (load_eta_ms, set_load_eta_ms) = signal(0u64);
        let (repo_key, set_repo_key) = signal(None);
        let (_, set_pending_navigation) = signal(None::<PendingNavigation>);

        let _ = (content, playback_version, load_progress, load_eta_ms);

        SyncContext {
            doc_id,
            client_id: None,
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
        }
    }

    #[test]
    fn write_ready_resend_blocks_when_native_runtime_readiness_fails() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let repo_id = RepoId::new_v4();
        let doc_id = DocId::from_u128(7);
        ws.mark_writer_ready(repo_id.to_string(), 17, "web-light-peer");
        ws.set_node_role_probe_failed_for_test();
        let ctx = write_ready_ctx(&ws, repo_id, doc_id, 17);

        handle_write_ready_message(&ctx, repo_id, None, 17);

        assert!(ws.drain_sent_for_test().is_empty());
    }

    #[test]
    fn write_ready_resend_sends_pending_edit_when_native_runtime_is_ready() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let repo_id = RepoId::new_v4();
        let doc_id = DocId::from_u128(7);
        ws.set_node_role_for_test("main");
        ws.mark_writer_ready(repo_id.to_string(), 17, "web-light-peer");
        let ctx = write_ready_ctx(&ws, repo_id, doc_id, 17);

        handle_write_ready_message(&ctx, repo_id, None, 17);

        let sent = ws.drain_sent_for_test();
        assert_eq!(sent.len(), 1);
        match &sent[0] {
            ClientMessage::Edit {
                doc_id: actual_doc_id,
                scope_nonce,
                ..
            } => {
                assert_eq!(*actual_doc_id, doc_id);
                assert_eq!(*scope_nonce, Some(17));
            }
            other => panic!("expected Edit, got {other:?}"),
        }
        assert_eq!(
            pending_count_for_doc(&ctx.pending_local_edits.get_untracked(), doc_id),
            1
        );
    }

    #[test]
    fn write_ready_resend_skips_pending_edit_from_stale_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let repo_id = RepoId::new_v4();
        let doc_id = DocId::from_u128(7);
        ws.set_node_role_for_test("main");
        ws.mark_writer_ready(repo_id.to_string(), 17, "web-light-peer");
        let ctx = write_ready_ctx_with_pending_scope(&ws, repo_id, doc_id, 17, 16);

        handle_write_ready_message(&ctx, repo_id, None, 17);

        assert!(ws.drain_sent_for_test().is_empty());
        assert_eq!(
            pending_count_for_doc(&ctx.pending_local_edits.get_untracked(), doc_id),
            1
        );
    }
}
