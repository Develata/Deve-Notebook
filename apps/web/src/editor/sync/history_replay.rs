//! plan_ref:
//!   - 03_rendering#large-document-runtime
//!   - 03_rendering#document-authority-bridge
//!
use super::context::SyncContext;
use super::live::apply_live_op;
use crate::editor::ffi::applyRemoteOpsBatch;
use crate::hooks::use_core::pending;
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::GetUntracked;

pub(super) fn merge_history_tail(
    history: &[ConfirmedOp],
    mut live_history: Vec<(u64, deve_core::models::Op)>,
) -> Vec<(u64, deve_core::models::Op)> {
    let mut last_seq = live_history.last().map(|(seq, _)| *seq).unwrap_or(0);
    for entry in history {
        if entry.seq > last_seq {
            live_history.push((entry.seq, entry.op.clone()));
            last_seq = entry.seq;
        }
    }
    live_history
}

pub(super) fn replay_pending_overlay(ctx: &SyncContext) {
    let Some(scope) = pending::PendingScope::from_repo_id_str(
        ctx.current_repo_id.get_untracked().as_deref(),
        ctx.current_scope_nonce.get_untracked(),
    ) else {
        return;
    };
    let ops = pending::cloned_ops_for_doc_in_scope(
        &ctx.pending_local_edits.get_untracked(),
        ctx.doc_id,
        scope,
    );
    if ops.is_empty() {
        return;
    }
    if let Ok(json) = serde_json::to_string(&ops) {
        applyRemoteOpsBatch(&json);
    }
}

pub(super) fn replay_buffered_live_ops(ctx: &SyncContext, expected_generation: u64) {
    if !ctx.is_generation_current(expected_generation) {
        return;
    }
    let mut buffered = ctx.drain_buffered_live_ops();
    buffered.sort_by_key(|entry| entry.seq);
    for entry in buffered {
        if !ctx.is_generation_current(expected_generation) {
            return;
        }
        apply_live_op(ctx, entry);
    }
}
