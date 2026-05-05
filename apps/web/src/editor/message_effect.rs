//! plan_ref:
//!   - 03_rendering#large-document-runtime
//!   - 05_network#web-ws-runtime
//!
use super::EditorStats;
use super::sync;
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
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
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub buffered_encrypted_ops: Arc<Mutex<Vec<EncryptedOp>>>,
    pub set_content: WriteSignal<String>,
    pub local_version: ReadSignal<u64>,
    pub set_local_version: WriteSignal<u64>,
    pub history: ReadSignal<Vec<(u64, Op)>>,
    pub set_history: WriteSignal<Vec<(u64, Op)>>,
    pub is_playback: ReadSignal<bool>,
    pub set_playback_version: WriteSignal<u64>,
    pub on_stats: Option<Callback<EditorStats>>,
    pub repo_key: ReadSignal<Option<RepoKey>>,
    pub set_repo_key: WriteSignal<Option<RepoKey>>,
}

pub fn setup_server_message_effect(ctx: ServerMessageEffectCtx) {
    let ServerMessageEffectCtx {
        ws,
        core,
        doc_id,
        open_request_id,
        session_generation,
        ready_generation,
        buffered_live_ops,
        buffered_encrypted_ops,
        set_content,
        local_version,
        set_local_version,
        history,
        set_history,
        is_playback,
        set_playback_version,
        on_stats,
        repo_key,
        set_repo_key,
    } = ctx;
    let (last_msg_seq, set_last_msg_seq) = signal(0u64);

    Effect::new(move |_| {
        let _ = ws.msg_seq.get();
        for (seq, connection_epoch, msg) in ws.messages_since(last_msg_seq.get_untracked()) {
            if !crate::api::is_current_connection_message(
                connection_epoch,
                ws.connection_epoch.get_untracked(),
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
                buffered_live_ops: buffered_live_ops.clone(),
                buffered_encrypted_ops: buffered_encrypted_ops.clone(),
                active_branch: core.active_branch,
                pending_branch_switch: core.pending_branch_switch,
                current_repo_id: core.current_repo_id,
                current_scope_nonce: core.current_scope_nonce,
                pending_repo_switch: core.pending_repo_switch,
                handshake_scope_nonce: core.handshake_scope_nonce,
                open_request_id,
                ws: &ws,
                set_content,
                pending_local_edits: core.pending_local_edits,
                set_pending_local_edits: core.set_pending_local_edits,
                local_version,
                set_local_version,
                history,
                set_history,
                is_playback,
                set_playback_version,
                set_load_state: core.set_load_state,
                set_load_progress: core.set_load_progress,
                set_load_eta_ms: core.set_load_eta_ms,
                on_stats,
                repo_key,
                set_repo_key,
            };
            sync::handle_server_message(msg, &ctx);
            set_last_msg_seq.set(seq);
        }
    });
}
