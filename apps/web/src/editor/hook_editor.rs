use super::EditorStats;
use super::buffered_ops::clear_sync_buffers;
use super::delta_input::{DeltaInputCtx, build_on_delta};
use super::ffi::{destroyEditor, setupCodeMirror};
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
use deve_core::models::DocId;
use deve_core::protocol::ConfirmedOp;
use deve_core::security::EncryptedOp;
use leptos::html::Div;
use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct EditorMountEffectCtx {
    pub doc_id: DocId,
    pub editor_ref: NodeRef<Div>,
    pub ws: WsService,
    pub core: EditorContext,
    pub is_playback: ReadSignal<bool>,
    pub local_version: ReadSignal<u64>,
    pub on_stats: Option<Callback<EditorStats>>,
    pub set_content: WriteSignal<String>,
}

pub fn setup_editor_mount_effect(ctx: EditorMountEffectCtx) {
    Effect::new(move |_| {
        let Some(element) = ctx.editor_ref.get() else {
            return;
        };
        let raw_element: &web_sys::HtmlElement = &element;
        let on_delta = build_on_delta(DeltaInputCtx {
            doc_id: ctx.doc_id,
            ws: ctx.ws.clone(),
            current_repo_id: ctx.core.current_repo_id,
            current_scope_nonce: ctx.core.current_scope_nonce,
            active_branch: ctx.core.active_branch,
            pending_branch_switch: ctx.core.pending_branch_switch,
            pending_repo_switch: ctx.core.pending_repo_switch,
            handshake_ready: ctx.core.handshake_ready,
            is_playback: ctx.is_playback,
            set_pending_local_edits: ctx.core.set_pending_local_edits,
            local_version: ctx.local_version,
            on_stats: ctx.on_stats,
            set_content: ctx.set_content,
        });
        setupCodeMirror(raw_element, &on_delta);
        let on_delta = StoredValue::new_local(Some(on_delta));
        on_cleanup(move || on_delta.update_value(|value| drop(value.take())));
    });
}

pub struct EditorCleanupCtx {
    pub session_generation: Arc<AtomicU64>,
    pub ready_generation: Arc<AtomicU64>,
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub buffered_encrypted_ops: Arc<Mutex<Vec<EncryptedOp>>>,
}

pub fn setup_editor_cleanup(ctx: EditorCleanupCtx) {
    on_cleanup(move || {
        let _ = ctx.session_generation.fetch_add(1, Ordering::Relaxed);
        ctx.ready_generation.store(0, Ordering::Relaxed);
        clear_sync_buffers(
            &ctx.buffered_live_ops,
            &ctx.buffered_encrypted_ops,
            "编辑器清理时忽略 buffered live ops",
            "编辑器清理时忽略 buffered encrypted ops",
        );
        destroyEditor();
    });
}
