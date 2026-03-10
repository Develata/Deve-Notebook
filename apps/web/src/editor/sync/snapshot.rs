// apps/web/src/editor/sync/snapshot.rs
//! Snapshot 消息处理: 接收文档快照并渐进式应用 delta ops

use super::context::SyncContext;
use super::snapshot_finish::{LoadFinish, emit_stats, finalize_load, now_ms};
use crate::editor::ffi::{applyRemoteContent, applyRemoteOpsBatch};
use crate::editor::prefetch::{PrefetchConfig, apply_ops_in_batches};
use deve_core::models::Op;
use leptos::prelude::*;

type BatchHandler = std::rc::Rc<dyn Fn(&[(u64, Op)])>;
type ProgressHandler = std::rc::Rc<dyn Fn(usize, usize, f64)>;

/// 处理 ServerMessage::Snapshot
///
/// # Pre-conditions
/// - `msg_doc_id` 已验证匹配 ctx.doc_id
///
/// # Post-conditions
/// - 编辑器内容更新为快照 + delta ops
/// - local_version 推进到最新 seq
pub(super) fn handle_snapshot(
    ctx: &SyncContext,
    request_id: u64,
    new_content: String,
    base_seq: u64,
    version: u64,
    delta_ops: Vec<(u64, Op)>,
) {
    let load_start = now_ms();

    leptos::logging::log!(
        "Received Snapshot: {} chars, Base: {}, Ver: {}, Pending: {}",
        new_content.len(),
        base_seq,
        version,
        delta_ops.len()
    );

    emit_stats(ctx.on_stats, &new_content);
    applyRemoteContent(&new_content);
    ctx.set_content.set(new_content);
    ctx.set_local_version.set(base_seq);
    ctx.set_playback_version.set(base_seq);
    ctx.set_load_state.set("partial".to_string());
    ctx.set_load_progress.set((0, delta_ops.len()));
    ctx.set_load_eta_ms.set(0);

    if delta_ops.is_empty() {
        finalize_load(ctx, version, load_start);
        return;
    }

    let apply_batch = build_apply_batch(ctx, request_id);
    let on_progress = build_progress_handler(ctx.set_load_progress, ctx.set_load_eta_ms);
    let finish = LoadFinish::from_ctx(ctx, version, load_start, request_id);
    let open_request_id = ctx.open_request_id;
    let on_done = std::rc::Rc::new(move || {
        if open_request_id.get_untracked() == request_id {
            finish.clone().complete();
        }
    });

    apply_ops_in_batches(
        delta_ops,
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

fn build_apply_batch(ctx: &SyncContext, request_id: u64) -> BatchHandler {
    let open_request_id = ctx.open_request_id;
    let set_local_version = ctx.set_local_version;
    let set_history = ctx.set_history;
    std::rc::Rc::new(move |batch: &[(u64, Op)]| {
        if open_request_id.get_untracked() != request_id {
            return;
        }
        let ops_only: Vec<Op> = batch.iter().map(|(_, op)| op.clone()).collect();
        if let Ok(json) = serde_json::to_string(&ops_only) {
            applyRemoteOpsBatch(&json);
        }
        if let Some((seq, _)) = batch.last() {
            set_local_version.set(*seq);
        }
        set_history.update(|history| {
            for (seq, op) in batch {
                history.push((*seq, op.clone()));
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
