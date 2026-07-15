//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 07_network#web-ws-runtime
//!
use super::EditorStats;
use super::sync;
use crate::api::IncomingBatch;
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
use crate::runtime::domain::EditorSyncFailure;
use crate::runtime::projection_recovery::ProjectionRecoveryCoordinator;
use deve_core::models::{DocId, Op};
use deve_core::protocol::ConfirmedOp;
use deve_core::security::{EncryptedOp, RepoKey};
use leptos::prelude::*;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ServerMessageEffectCtx {
    pub ws: WsService,
    pub core: EditorContext,
    pub doc_id: DocId,
    pub open_request_id: ReadSignal<u64>,
    pub session_generation: Arc<AtomicU64>,
    pub ready_generation: Arc<AtomicU64>,
    pub pending_resend_generation: Arc<AtomicU64>,
    pub projection_recovery: ProjectionRecoveryCoordinator,
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub buffered_encrypted_ops: Arc<Mutex<Vec<EncryptedOp>>>,
    pub set_content: WriteSignal<String>,
    pub content: ReadSignal<String>,
    pub local_version: ReadSignal<u64>,
    pub set_local_version: WriteSignal<u64>,
    pub history: ReadSignal<Vec<(u64, Op)>>,
    pub set_history: WriteSignal<Vec<(u64, Op)>>,
    pub is_playback: ReadSignal<bool>,
    pub set_playback_version: WriteSignal<u64>,
    pub on_stats: Option<Callback<EditorStats>>,
    pub repo_key: ReadSignal<Option<RepoKey>>,
    pub set_repo_key: WriteSignal<Option<RepoKey>>,
    pub set_editor_sync_failure: WriteSignal<Option<EditorSyncFailure>>,
    pub snapshot_reopen_attempted: ReadSignal<bool>,
    pub set_snapshot_reopen_attempted: WriteSignal<bool>,
    pub set_open_request_id: WriteSignal<u64>,
}

pub fn setup_server_message_effect(ctx: ServerMessageEffectCtx) {
    let ServerMessageEffectCtx {
        ws,
        core,
        doc_id,
        open_request_id,
        session_generation,
        ready_generation,
        pending_resend_generation,
        projection_recovery,
        buffered_live_ops,
        buffered_encrypted_ops,
        set_content,
        content,
        local_version,
        set_local_version,
        history,
        set_history,
        is_playback,
        set_playback_version,
        on_stats,
        repo_key,
        set_repo_key,
        set_editor_sync_failure,
        snapshot_reopen_attempted,
        set_snapshot_reopen_attempted,
        set_open_request_id,
    } = ctx;
    // The editor consumer is mounted after the global consumer. Older retained
    // messages predate this document session and are not part of its cursor.
    let (last_msg_seq, set_last_msg_seq) = signal(ws.msg_seq.get_untracked());

    Effect::new(move |_| {
        let _ = ws.msg_seq.get();
        let current_connection_epoch = ws.connection_epoch.get_untracked();
        if ws.reconnect_for_resync_pending(current_connection_epoch) {
            set_last_msg_seq.set(ws.msg_seq.get_untracked());
            return;
        }
        let messages = match ws.messages_since(last_msg_seq.get_untracked()) {
            IncomingBatch::Messages(messages) => messages,
            IncomingBatch::Gap { latest_seq } => {
                set_last_msg_seq.set(latest_seq);
                core.set_load_state
                    .set(crate::runtime::domain::LoadPhase::Resyncing);
                ws.request_reconnect_for_resync(current_connection_epoch);
                return;
            }
        };
        for (seq, connection_epoch, msg) in messages {
            if !crate::api::is_current_connection_message(
                connection_epoch,
                current_connection_epoch,
            ) {
                set_last_msg_seq.set(seq);
                continue;
            }
            let ctx = sync::context::SyncContext {
                doc_id,
                client_id: ws.writer_client_id_for(
                    core.current_repo_id.get_untracked().as_deref(),
                    Some(core.current_scope_nonce.get_untracked()),
                ),
                session_generation: session_generation.clone(),
                ready_generation: ready_generation.clone(),
                pending_resend_generation: pending_resend_generation.clone(),
                projection_recovery: projection_recovery.clone(),
                buffered_live_ops: buffered_live_ops.clone(),
                buffered_encrypted_ops: buffered_encrypted_ops.clone(),
                active_branch: core.active_branch,
                current_doc: core.current_doc,
                pending_branch_switch: core.pending_branch_switch,
                current_repo_id: core.current_repo_id,
                current_scope_nonce: core.current_scope_nonce,
                pending_repo_switch: core.pending_repo_switch,
                handshake_scope_nonce: core.handshake_scope_nonce,
                load_state: core.load_state,
                is_spectator: core.is_spectator,
                handshake_ready: core.handshake_ready,
                open_request_id,
                set_open_request_id,
                ws: &ws,
                set_content,
                content,
                pending_local_edits: core.pending_local_edits,
                set_pending_local_edits: core.set_pending_local_edits,
                set_pending_navigation: core.set_pending_navigation,
                local_version,
                set_local_version,
                history,
                set_history,
                is_playback,
                set_playback_version,
                set_load_state: core.set_load_state,
                set_load_progress: core.set_load_progress,
                set_load_eta_ms: core.set_load_eta_ms,
                set_editor_sync_failure,
                snapshot_reopen_attempted,
                set_snapshot_reopen_attempted,
                on_stats,
                repo_key,
                set_repo_key,
            };
            sync::handle_server_message(msg, &ctx);
            set_last_msg_seq.set(seq);
        }
    });
}
