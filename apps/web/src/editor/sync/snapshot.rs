// apps/web/src/editor/sync/snapshot.rs
//! Snapshot 消息处理: 接收文档快照并渐进式应用 delta ops

use super::context::SyncContext;
use crate::editor::EditorStats;
use crate::editor::ffi::{applyRemoteContent, applyRemoteOpsBatch, getEditorContent};
use crate::editor::prefetch::{PrefetchConfig, apply_ops_in_batches};
use deve_core::models::Op;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

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
    new_content: String,
    base_seq: u64,
    version: u64,
    delta_ops: Vec<(u64, Op)>,
) {
    let load_start = now_ms();
    let doc_id = ctx.doc_id;

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

    // 初始化回放范围
    ctx.set_playback_version.set(base_seq);
    ctx.set_load_state.set("partial".to_string());
    ctx.set_load_progress.set((0, delta_ops.len()));
    ctx.set_load_eta_ms.set(0);

    if delta_ops.is_empty() {
        finalize_load(ctx, version, load_start);
        return;
    }

    let ws = ctx.ws.clone();
    let set_local_version = ctx.set_local_version;
    let set_history = ctx.set_history;
    let set_load_progress = ctx.set_load_progress;
    let set_load_eta_ms = ctx.set_load_eta_ms;
    let set_content = ctx.set_content;
    let set_playback_version = ctx.set_playback_version;
    let set_load_state = ctx.set_load_state;
    let on_stats = ctx.on_stats;

    let apply_batch = std::rc::Rc::new(move |batch: &[(u64, Op)]| {
        let ops_only: Vec<Op> = batch.iter().map(|(_, op)| op.clone()).collect();
        if let Ok(json) = serde_json::to_string(&ops_only) {
            applyRemoteOpsBatch(&json);
        }
        if let Some((seq, _)) = batch.last() {
            set_local_version.set(*seq);
        }
        set_history.update(|h| {
            for (seq, op) in batch {
                h.push((*seq, op.clone()));
            }
        });
    });

    let elapsed_total = std::rc::Rc::new(std::cell::RefCell::new(0.0));
    let on_progress = {
        let elapsed_total = elapsed_total.clone();
        std::rc::Rc::new(move |done: usize, total: usize, batch_ms: f64| {
            set_load_progress.set((done, total));
            *elapsed_total.borrow_mut() += batch_ms;
            if done > 0 {
                let per_op = *elapsed_total.borrow() / done as f64;
                let remaining = (total - done) as f64 * per_op;
                set_load_eta_ms.set(remaining as u64);
            }
        })
    };

    let on_done = std::rc::Rc::new(move || {
        let txt = getEditorContent();
        emit_stats(on_stats, &txt);
        set_content.set(txt);
        set_playback_version.set(version);
        set_load_state.set("ready".to_string());
        set_load_progress.set((0, 0));
        set_load_eta_ms.set(0);
        leptos::logging::log!(
            "Snapshot load complete: doc={}, elapsed_ms={}",
            doc_id,
            (now_ms() - load_start) as u64
        );
        ws.send(ClientMessage::RequestHistory { doc_id });
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

/// 快照无 delta ops 时直接完成加载
fn finalize_load(ctx: &SyncContext, version: u64, load_start: f64) {
    ctx.set_local_version.set(version);
    ctx.set_playback_version.set(version);
    ctx.set_load_state.set("ready".to_string());
    ctx.set_load_progress.set((0, 0));
    ctx.set_load_eta_ms.set(0);
    leptos::logging::log!(
        "Snapshot load complete: doc={}, elapsed_ms={}",
        ctx.doc_id,
        (now_ms() - load_start) as u64
    );
    ctx.ws
        .send(ClientMessage::RequestHistory { doc_id: ctx.doc_id });
}

fn emit_stats(on_stats: Option<Callback<EditorStats>>, text: &str) {
    if let Some(cb) = on_stats {
        cb.run(EditorStats {
            chars: text.len(),
            words: text.split_whitespace().count(),
            lines: text.lines().count(),
        });
    }
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}
