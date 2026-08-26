// apps/web/src/editor/sync/snapshot.rs
//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 07_network#web-ws-runtime
//!
//! Snapshot 消息处理: 接收文档快照并渐进式应用 delta ops

use super::context::SyncContext;
use super::snapshot_apply::{SnapshotApplySignals, build_apply_batch, build_progress_handler};
use super::snapshot_finish::{LoadFinish, emit_stats, finalize_load, now_ms};
use super::snapshot_gate::{SnapshotRequestGate, SnapshotRequestGateInput};
use crate::editor::buffered_ops::clear_sync_buffers;
use crate::editor::ffi::{applyRemoteContent, set_read_only};
use crate::editor::hook_open::advance_session_generation;
use crate::editor::prefetch::{CancelBatchFn, PrefetchConfig, apply_ops_in_batches};
use crate::runtime::domain::{EditorSyncFailure, EditorSyncFailureCode, LoadPhase};
use deve_core::models::{Op, PeerId, RepoId};
use deve_core::protocol::{ClientMessage, ConfirmedOp};
use leptos::prelude::*;
use std::sync::atomic::Ordering;

#[cfg(test)]
pub(super) use super::snapshot_gate::{SnapshotRequestMatch, snapshot_request_matches};

#[derive(Clone)]
pub(super) struct SnapshotMessage {
    pub expected_generation: u64,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub request_id: u64,
    pub new_content: String,
    pub base_seq: u64,
    pub version: u64,
    pub delta_ops: Vec<ConfirmedOp>,
}

/// 处理 ServerMessage::Snapshot
///
/// # Pre-conditions
/// - `msg_doc_id` 已验证匹配 ctx.doc_id
///
/// # Post-conditions
/// - 编辑器内容更新为快照 + delta ops
/// - local_version 推进到最新 seq
pub(super) fn handle_snapshot(ctx: &SyncContext, message: SnapshotMessage) {
    if !ctx.is_generation_current(message.expected_generation) {
        return;
    }
    let load_start = now_ms();
    let gate = SnapshotRequestGate::new(SnapshotRequestGateInput {
        open_request_id: ctx.open_request_id,
        current_repo_id: ctx.current_repo_id,
        pending_repo_switch: ctx.pending_repo_switch,
        active_branch: ctx.active_branch,
        pending_branch_switch: ctx.pending_branch_switch,
        current_scope_nonce: ctx.current_scope_nonce,
        session_generation: ctx.session_generation.clone(),
        expected_generation: message.expected_generation,
        repo_id: message.repo_id,
        branch: message.branch.clone(),
        request_id: message.request_id,
        scope_nonce: ctx.current_scope_nonce.get_untracked(),
    });

    leptos::logging::log!(
        "Received Snapshot: {} chars, Base: {}, Ver: {}, Pending: {}",
        message.new_content.len(),
        message.base_seq,
        message.version,
        message.delta_ops.len()
    );

    if !applyRemoteContent(&message.new_content) {
        leptos::logging::warn!(
            "Snapshot apply blocked: editor content bridge unavailable for doc={}",
            ctx.doc_id
        );
        retry_initial_snapshot_once(ctx, &gate);
        return;
    }
    emit_stats(ctx.on_stats, &message.new_content);
    let fallback_base = (!message.delta_ops.is_empty()).then(|| message.new_content.clone());
    ctx.set_content.set(message.new_content);
    ctx.set_local_version.set(message.base_seq);
    ctx.set_playback_version.set(message.base_seq);
    ctx.set_load_state.set(LoadPhase::Partial);
    ctx.set_load_progress.set((0, message.delta_ops.len()));
    ctx.set_load_eta_ms.set(0);

    if message.delta_ops.is_empty() {
        if ctx.is_generation_current(message.expected_generation) {
            finalize_load(ctx, message.version, load_start);
        }
        return;
    }

    let apply_batch = build_apply_batch(
        SnapshotApplySignals {
            set_local_version: ctx.set_local_version,
            set_history: ctx.set_history,
        },
        gate.clone(),
    );
    let on_cancel = build_delta_failure_fallback(
        ctx,
        &gate,
        DeltaFailureFallback {
            base_content: fallback_base.expect("non-empty delta retains snapshot base"),
            version: message.version,
            finish: LoadFinish::from_ctx(ctx, message.version, load_start, message.request_id),
        },
    );
    let on_progress = build_progress_handler(ctx.set_load_progress, ctx.set_load_eta_ms);
    let finish = LoadFinish::from_ctx(ctx, message.version, load_start, message.request_id);
    let on_done = std::rc::Rc::new(move || {
        if gate.matches() {
            finish.clone().complete();
        }
    });

    apply_ops_in_batches(
        message.delta_ops,
        PrefetchConfig {
            target_ms: 8.0,
            initial_batch: 16,
            max_batch: 256,
        },
        apply_batch,
        on_cancel,
        on_progress,
        on_done,
    );
}

struct DeltaFailureFallback {
    base_content: String,
    version: u64,
    finish: LoadFinish,
}

fn build_delta_failure_fallback(
    ctx: &SyncContext,
    gate: &SnapshotRequestGate,
    fallback: DeltaFailureFallback,
) -> CancelBatchFn<ConfirmedOp> {
    let gate = gate.clone();
    let doc_id = ctx.doc_id;
    let set_local_version = ctx.set_local_version;
    let set_history = ctx.set_history;
    let set_load_state = ctx.set_load_state;
    let set_load_progress = ctx.set_load_progress;
    let set_load_eta_ms = ctx.set_load_eta_ms;
    let set_editor_sync_failure = ctx.set_editor_sync_failure;
    let open_request_id = ctx.open_request_id;
    let session_generation = ctx.session_generation.clone();
    let ready_generation = ctx.ready_generation.clone();
    std::rc::Rc::new(move |delta_ops| {
        if !gate.matches() {
            return;
        }
        if let Some(full_content) =
            reconstruct_full_snapshot_content(&fallback.base_content, delta_ops)
        {
            leptos::logging::warn!(
                "Snapshot delta batch apply failed; applying reconstructed full snapshot fallback for doc={doc_id}"
            );
            if applyRemoteContent(&full_content) {
                set_local_version.set(fallback.version);
                set_history.set(confirmed_history(delta_ops));
                fallback.finish.clone().complete_with_content(full_content);
                return;
            }
            leptos::logging::warn!(
                "Snapshot reconstructed fallback blocked: editor content bridge unavailable for doc={doc_id}"
            );
        }
        leptos::logging::warn!("Snapshot delta batch apply failed closed for doc={doc_id}");
        set_read_only(true);
        ready_generation.store(0, Ordering::Relaxed);
        set_load_state.set(LoadPhase::Error);
        set_load_progress.set((0, 0));
        set_load_eta_ms.set(0);
        set_editor_sync_failure.set(Some(EditorSyncFailure::new(
            EditorSyncFailureCode::DeltaReplay,
            session_generation.load(Ordering::Relaxed),
            open_request_id.get_untracked(),
        )));
    })
}

fn retry_initial_snapshot_once(ctx: &SyncContext, gate: &SnapshotRequestGate) {
    if !gate.matches() {
        return;
    }
    if !initial_snapshot_may_auto_reopen(ctx.snapshot_reopen_attempted.get_untracked()) {
        ctx.fail_editor_sync(EditorSyncFailureCode::SnapshotApply);
        return;
    }

    let request_id = advance_session_generation(&ctx.session_generation);
    ctx.projection_recovery.mark_generation(request_id);
    ctx.set_snapshot_reopen_attempted.set(true);
    ctx.set_editor_sync_failure.set(None);
    ctx.ready_generation.store(0, Ordering::Relaxed);
    clear_sync_buffers(
        &ctx.buffered_live_ops,
        &ctx.buffered_encrypted_ops,
        "snapshot auto-reopen clears buffered live ops",
        "snapshot auto-reopen clears buffered encrypted ops",
    );
    set_read_only(true);
    ctx.set_open_request_id.set(request_id);
    ctx.set_local_version.set(0);
    ctx.set_history.set(Vec::new());
    ctx.set_playback_version.set(0);
    ctx.set_load_state
        .set(if ctx.projection_recovery.is_active() {
            LoadPhase::Resyncing
        } else {
            LoadPhase::Loading
        });
    ctx.set_load_progress.set((0, 0));
    ctx.set_load_eta_ms.set(0);
    ctx.ws.send(ClientMessage::OpenDoc {
        doc_id: ctx.doc_id,
        request_id,
        scope_nonce: Some(ctx.current_scope_nonce.get_untracked()),
    });
}

fn initial_snapshot_may_auto_reopen(attempted: bool) -> bool {
    !attempted
}

fn reconstruct_full_snapshot_content(base: &str, delta_ops: &[ConfirmedOp]) -> Option<String> {
    deve_core::state::try_apply_content_ops(base, delta_ops.iter().map(|entry| &entry.op))
}

fn confirmed_history(delta_ops: &[ConfirmedOp]) -> Vec<(u64, Op)> {
    delta_ops
        .iter()
        .map(|entry| (entry.seq, entry.op.clone()))
        .collect()
}

#[cfg(test)]
mod tests;
