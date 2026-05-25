//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!
use super::buffered_ops::clear_sync_buffers;
use super::open_scope::{OpenDocScope, OpenRequestKey, open_request_key};
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::EditorContext;
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ClientMessage, ConfirmedOp};
use deve_core::security::EncryptedOp;
use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct OpenDocEffectCtx {
    pub ws: WsService,
    pub core: EditorContext,
    pub doc_id: DocId,
    pub last_open_request_key: ReadSignal<Option<OpenRequestKey>>,
    pub set_last_open_request_key: WriteSignal<Option<OpenRequestKey>>,
    pub session_generation: Arc<AtomicU64>,
    pub ready_generation: Arc<AtomicU64>,
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub buffered_encrypted_ops: Arc<Mutex<Vec<EncryptedOp>>>,
    pub set_local_version: WriteSignal<u64>,
    pub set_open_request_id: WriteSignal<u64>,
    pub set_history: WriteSignal<Vec<(u64, Op)>>,
}

pub fn setup_open_doc_effect(ctx: OpenDocEffectCtx) {
    Effect::new(move |_| {
        let Some(open_key) = current_open_key(&ctx) else {
            ctx.set_last_open_request_key.set(None);
            return;
        };
        if ctx.last_open_request_key.get_untracked() == Some(open_key) {
            return;
        }
        ctx.set_last_open_request_key.set(Some(open_key));
        let request_id = next_request_id();
        let _ = ctx.session_generation.fetch_add(1, Ordering::Relaxed);
        ctx.ready_generation.store(0, Ordering::Relaxed);
        clear_sync_buffers(
            &ctx.buffered_live_ops,
            &ctx.buffered_encrypted_ops,
            "清空 buffered live ops",
            "清空 buffered encrypted ops",
        );
        super::ffi::set_read_only(true);
        ctx.set_local_version.set(0);
        ctx.set_open_request_id.set(request_id);
        ctx.set_history.set(Vec::new());
        ctx.core.set_doc_version.set(0);
        ctx.core.set_playback_version.set(0);
        ctx.core.set_load_state.set("loading".to_string());
        ctx.core.set_load_progress.set((0, 0));
        ctx.core.set_load_eta_ms.set(0);
        leptos::logging::log!(
            "OpenDoc send: doc={}, request_id={}, scope_nonce={}",
            open_key.doc_id,
            request_id,
            open_key.scope_nonce
        );
        ctx.ws.send(ClientMessage::OpenDoc {
            doc_id: open_key.doc_id,
            request_id,
            scope_nonce: Some(open_key.scope_nonce),
        });
    });
}

fn current_open_key(ctx: &OpenDocEffectCtx) -> Option<OpenRequestKey> {
    open_request_key(
        OpenDocScope {
            doc_id: ctx.doc_id,
            docs: &ctx.core.docs.get(),
            doc_selected: ctx.core.current_doc.get() == Some(ctx.doc_id),
            has_repo_scope: ctx.core.current_repo_id.get().is_some(),
            branch_switch_idle: ctx.core.pending_branch_switch.get().is_none(),
            repo_switch_idle: ctx.core.pending_repo_switch.get().is_none(),
        },
        ctx.ws.status.get() == ConnectionStatus::Connected,
        ctx.core.current_scope_nonce.get(),
    )
}

fn next_request_id() -> u64 {
    js_sys::Math::floor(js_sys::Math::random() * u64::MAX as f64) as u64
}
