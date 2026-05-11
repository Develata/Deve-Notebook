// apps/web/src/editor/sync/snapshot.rs
//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 05_network#web-ws-runtime
//!
//! Snapshot 消息处理: 接收文档快照并渐进式应用 delta ops

use super::context::SyncContext;
use super::snapshot_apply::{SnapshotApplySignals, build_apply_batch, build_progress_handler};
use super::snapshot_finish::{LoadFinish, emit_stats, finalize_load, now_ms};
use super::snapshot_gate::{SnapshotRequestGate, SnapshotRequestGateInput};
use crate::editor::ffi::applyRemoteContent;
use crate::editor::prefetch::{PrefetchConfig, apply_ops_in_batches};
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::*;

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

    emit_stats(ctx.on_stats, &message.new_content);
    applyRemoteContent(&message.new_content);
    ctx.set_content.set(message.new_content);
    ctx.set_local_version.set(message.base_seq);
    ctx.set_playback_version.set(message.base_seq);
    ctx.set_load_state.set("partial".to_string());
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
        on_progress,
        on_done,
    );
}

#[cfg(test)]
mod tests;
