//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
use super::EditorStats;
use super::buffered_ops::clear_sync_buffers;
use super::delta_input::{DeltaInputCtx, build_on_delta};
use super::ffi::{destroyEditor, setupCodeMirror};
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
use crate::runtime::domain::LoadPhase;
use deve_core::models::DocId;
use deve_core::protocol::ConfirmedOp;
use deve_core::security::EncryptedOp;
use leptos::html::Div;
use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use wasm_bindgen::closure::Closure;

pub struct EditorMountEffectCtx {
    pub doc_id: DocId,
    pub editor_ref: NodeRef<Div>,
    pub ws: WsService,
    pub core: EditorContext,
    pub is_playback: ReadSignal<bool>,
    pub local_version: ReadSignal<u64>,
    pub on_stats: Option<Callback<EditorStats>>,
    pub set_content: WriteSignal<String>,
    pub set_editor_ready: WriteSignal<bool>,
}

pub fn setup_editor_mount_effect(ctx: EditorMountEffectCtx) {
    Effect::new(move |_| {
        let Some(element) = ctx.editor_ref.get() else {
            return;
        };
        let raw_element: &web_sys::HtmlElement = &element;
        ctx.set_editor_ready.set(false);
        let on_delta = build_on_delta(DeltaInputCtx {
            doc_id: ctx.doc_id,
            ws: ctx.ws.clone(),
            current_repo_id: ctx.core.current_repo_id,
            current_scope_nonce: ctx.core.current_scope_nonce,
            active_branch: ctx.core.active_branch,
            pending_branch_switch: ctx.core.pending_branch_switch,
            pending_repo_switch: ctx.core.pending_repo_switch,
            load_state: ctx.core.load_state,
            is_spectator: ctx.core.is_spectator,
            handshake_ready: ctx.core.handshake_ready,
            is_playback: ctx.is_playback,
            set_pending_local_edits: ctx.core.set_pending_local_edits,
            local_version: ctx.local_version,
            on_stats: ctx.on_stats,
            set_content: ctx.set_content,
        });
        let set_editor_ready = ctx.set_editor_ready;
        let set_load_state = ctx.core.set_load_state;
        let set_load_progress = ctx.core.set_load_progress;
        let set_load_eta_ms = ctx.core.set_load_eta_ms;
        let ready_element = raw_element.clone();
        let on_ready = Closure::wrap(Box::new(move || {
            // `on_ready` is invoked only after the JS adapter has admitted this
            // exact connected host as the active owner. A detached stale effect
            // must not mutate the shared projection state of a replacement view.
            super::ffi::set_read_only_for_host(&ready_element, true);
            begin_editor_projection_load(set_load_state, set_load_progress, set_load_eta_ms);
            set_editor_ready.set(true);
        }) as Box<dyn FnMut()>);
        if !setupCodeMirror(raw_element, &on_delta, &on_ready) {
            leptos::logging::warn!("CodeMirror setup blocked: editor bridge unavailable");
            return;
        }
        let callbacks = StoredValue::new_local(Some((on_delta, on_ready)));
        let cleanup_element = raw_element.clone();
        on_cleanup(move || {
            let _ = destroyEditor(&cleanup_element);
            callbacks.update_value(|value| drop(value.take()));
        });
    });
}

fn begin_editor_projection_load(
    set_load_state: WriteSignal<LoadPhase>,
    set_load_progress: WriteSignal<(usize, usize)>,
    set_load_eta_ms: WriteSignal<u64>,
) {
    set_load_state.set(LoadPhase::Loading);
    set_load_progress.set((0, 0));
    set_load_eta_ms.set(0);
}

pub struct EditorCleanupCtx {
    pub session_generation: Arc<AtomicU64>,
    pub ready_generation: Arc<AtomicU64>,
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub buffered_encrypted_ops: Arc<Mutex<Vec<EncryptedOp>>>,
    pub set_editor_ready: WriteSignal<bool>,
}

pub fn setup_editor_cleanup(ctx: EditorCleanupCtx) {
    on_cleanup(move || {
        ctx.set_editor_ready.set(false);
        let _ = ctx.session_generation.fetch_add(1, Ordering::Relaxed);
        ctx.ready_generation.store(0, Ordering::Relaxed);
        clear_sync_buffers(
            &ctx.buffered_live_ops,
            &ctx.buffered_encrypted_ops,
            "编辑器清理时忽略 buffered live ops",
            "编辑器清理时忽略 buffered encrypted ops",
        );
    });
}

#[cfg(test)]
mod tests {
    use super::begin_editor_projection_load;
    use crate::runtime::domain::LoadPhase;
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn admitted_editor_mount_revokes_stale_ready_projection_before_open_doc() {
        let owner = Owner::new();
        owner.set();
        let (load_state, set_load_state) = signal(LoadPhase::Ready);
        let (load_progress, set_load_progress) = signal((7usize, 9usize));
        let (load_eta_ms, set_load_eta_ms) = signal(42u64);

        begin_editor_projection_load(set_load_state, set_load_progress, set_load_eta_ms);

        assert_eq!(load_state.get_untracked(), LoadPhase::Loading);
        assert_eq!(load_progress.get_untracked(), (0, 0));
        assert_eq!(load_eta_ms.get_untracked(), 0);
    }
}
