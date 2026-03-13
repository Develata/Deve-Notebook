// apps/web/src/editor/sync/snapshot.rs
//! Snapshot 消息处理: 接收文档快照并渐进式应用 delta ops

use super::context::SyncContext;
use super::scope::matches_scope;
use super::snapshot_finish::{LoadFinish, emit_stats, finalize_load, now_ms};
use crate::editor::ffi::{applyRemoteContent, applyRemoteOpsBatch};
use crate::editor::prefetch::{PrefetchConfig, apply_ops_in_batches};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

type BatchHandler = std::rc::Rc<dyn Fn(&[ConfirmedOp])>;
type ProgressHandler = std::rc::Rc<dyn Fn(usize, usize, f64)>;

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

#[derive(Clone)]
struct SnapshotRequestGate {
    open_request_id: ReadSignal<u64>,
    current_repo_id: ReadSignal<Option<String>>,
    pending_repo_switch: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    session_generation: Arc<AtomicU64>,
    expected_generation: u64,
    repo_id: RepoId,
    branch: Option<PeerId>,
    request_id: u64,
}

#[derive(Clone, Copy)]
struct SnapshotApplySignals {
    set_local_version: WriteSignal<u64>,
    set_history: WriteSignal<Vec<(u64, deve_core::models::Op)>>,
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
    let gate = SnapshotRequestGate {
        open_request_id: ctx.open_request_id,
        current_repo_id: ctx.current_repo_id,
        pending_repo_switch: ctx.pending_repo_switch,
        active_branch: ctx.active_branch,
        pending_branch_switch: ctx.pending_branch_switch,
        session_generation: ctx.session_generation.clone(),
        expected_generation: message.expected_generation,
        repo_id: message.repo_id,
        branch: message.branch.clone(),
        request_id: message.request_id,
    };

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

fn build_apply_batch(signals: SnapshotApplySignals, gate: SnapshotRequestGate) -> BatchHandler {
    std::rc::Rc::new(move |batch: &[ConfirmedOp]| {
        if !gate.matches() {
            return;
        }
        let ops_only: Vec<_> = batch.iter().map(|entry| entry.op.clone()).collect();
        if let Ok(json) = serde_json::to_string(&ops_only) {
            applyRemoteOpsBatch(&json);
        }
        if let Some(entry) = batch.last() {
            signals.set_local_version.set(entry.seq);
        }
        signals.set_history.update(|history| {
            for entry in batch {
                history.push((entry.seq, entry.op.clone()));
            }
        });
    })
}

fn build_progress_handler(
    set_load_progress: WriteSignal<(usize, usize)>,
    set_load_eta_ms: WriteSignal<u64>,
) -> ProgressHandler {
    let elapsed_total = std::rc::Rc::new(std::cell::RefCell::new(0.0));
    std::rc::Rc::new(move |done: usize, total: usize, batch_ms: f64| {
        set_load_progress.set((done, total));
        *elapsed_total.borrow_mut() += batch_ms;
        if done > 0 {
            let per_op = *elapsed_total.borrow() / done as f64;
            let remaining = (total - done) as f64 * per_op;
            set_load_eta_ms.set(remaining as u64);
        }
    })
}

fn snapshot_request_matches(args: SnapshotRequestMatch) -> bool {
    args.open_request_id == args.request_id
        && args.current_generation == args.expected_generation
        && matches_scope(
            args.current_repo_id,
            args.pending_repo_switch,
            args.active_branch,
            args.pending_branch_switch,
            Some(args.repo_id),
            args.branch,
        )
}

#[derive(Clone)]
struct SnapshotRequestMatch {
    open_request_id: u64,
    request_id: u64,
    current_repo_id: Option<String>,
    pending_repo_switch: Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    current_generation: u64,
    expected_generation: u64,
    repo_id: RepoId,
    branch: Option<PeerId>,
}

impl SnapshotRequestGate {
    fn matches(&self) -> bool {
        snapshot_request_matches(SnapshotRequestMatch {
            open_request_id: self.open_request_id.get_untracked(),
            request_id: self.request_id,
            current_repo_id: self.current_repo_id.get_untracked(),
            pending_repo_switch: self.pending_repo_switch.get_untracked(),
            active_branch: self.active_branch.get_untracked(),
            pending_branch_switch: self.pending_branch_switch.get_untracked(),
            current_generation: self.session_generation.load(Ordering::Relaxed),
            expected_generation: self.expected_generation,
            repo_id: self.repo_id,
            branch: self.branch.clone(),
        })
    }
}

#[cfg(test)]
#[path = "snapshot_test.rs"]
mod tests;
