//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 07_network#web-ws-runtime
//!
use super::EditorStats;
use super::handshake_reset::{HandshakeResetCtx, setup_handshake_reset_effect};
use super::hook_editor::{
    EditorCleanupCtx, EditorMountEffectCtx, setup_editor_cleanup, setup_editor_mount_effect,
};
use super::hook_open::{OpenDocEffectCtx, setup_open_doc_effect};
use super::hook_playback::{PlaybackEffectCtx, setup_playback_effect};
use super::hook_runtime::EditorRuntime;
use super::message_effect;
use super::request_key::setup_request_key_effect;
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
use deve_core::models::DocId;
use leptos::html::Div;
use leptos::prelude::*;

pub(super) fn setup_editor_effects(
    runtime: &EditorRuntime,
    ws: WsService,
    core: EditorContext,
    doc_id: DocId,
    editor_ref: NodeRef<Div>,
    on_stats: Option<Callback<EditorStats>>,
) {
    setup_open_doc_effect(OpenDocEffectCtx {
        ws: ws.clone(),
        core: core.clone(),
        doc_id,
        last_open_request_key: runtime.last_open_request_key,
        set_last_open_request_key: runtime.set_last_open_request_key,
        editor_ready: runtime.editor_ready,
        retry_nonce: runtime.retry_nonce,
        session_generation: runtime.session_generation.clone(),
        ready_generation: runtime.ready_generation.clone(),
        buffered_live_ops: runtime.buffered_live_ops.clone(),
        buffered_encrypted_ops: runtime.buffered_encrypted_ops.clone(),
        set_local_version: runtime.set_local_version,
        set_open_request_id: runtime.set_open_request_id,
        set_history: runtime.set_history,
        set_editor_sync_failure: runtime.set_editor_sync_failure,
        set_snapshot_reopen_attempted: runtime.set_snapshot_reopen_attempted,
    });
    setup_request_key_effect(ws.clone(), core.clone(), runtime.set_repo_key);
    Effect::new({
        let local_version = runtime.local_version;
        let set_doc_version = runtime.set_doc_version;
        move |_| set_doc_version.set(local_version.get())
    });
    message_effect::setup_server_message_effect(message_effect::ServerMessageEffectCtx {
        ws: ws.clone(),
        core: core.clone(),
        doc_id,
        open_request_id: runtime.open_request_id,
        session_generation: runtime.session_generation.clone(),
        ready_generation: runtime.ready_generation.clone(),
        buffered_live_ops: runtime.buffered_live_ops.clone(),
        buffered_encrypted_ops: runtime.buffered_encrypted_ops.clone(),
        set_content: runtime.set_content,
        content: runtime.content,
        local_version: runtime.local_version,
        set_local_version: runtime.set_local_version,
        history: runtime.history,
        set_history: runtime.set_history,
        is_playback: runtime.is_playback,
        set_playback_version: runtime.set_playback_version,
        on_stats: on_stats.clone(),
        repo_key: runtime.repo_key,
        set_repo_key: runtime.set_repo_key,
        set_editor_sync_failure: runtime.set_editor_sync_failure,
        snapshot_reopen_attempted: runtime.snapshot_reopen_attempted,
        set_snapshot_reopen_attempted: runtime.set_snapshot_reopen_attempted,
        set_open_request_id: runtime.set_open_request_id,
    });
    setup_handshake_reset_effect(HandshakeResetCtx {
        ws: ws.clone(),
        core: core.clone(),
        ready_generation: runtime.ready_generation.clone(),
        buffered_live_ops: runtime.buffered_live_ops.clone(),
        buffered_encrypted_ops: runtime.buffered_encrypted_ops.clone(),
        set_repo_key: runtime.set_repo_key,
    });
    setup_editor_mount_effect(EditorMountEffectCtx {
        doc_id,
        editor_ref,
        ws: ws.clone(),
        core: core.clone(),
        is_playback: runtime.is_playback,
        local_version: runtime.local_version,
        on_stats,
        set_content: runtime.set_content,
        set_editor_ready: runtime.set_editor_ready,
    });
    setup_editor_cleanup(EditorCleanupCtx {
        session_generation: runtime.session_generation.clone(),
        ready_generation: runtime.ready_generation.clone(),
        buffered_live_ops: runtime.buffered_live_ops.clone(),
        buffered_encrypted_ops: runtime.buffered_encrypted_ops.clone(),
        set_editor_ready: runtime.set_editor_ready,
    });
    setup_playback_effect(PlaybackEffectCtx {
        doc_id,
        history: runtime.history,
        playback_version: runtime.playback_version,
        local_version: runtime.local_version,
        set_is_playback: runtime.set_is_playback,
    });
}
