use super::context::SyncContext;
use crate::editor::EditorStats;
use crate::editor::ffi::{applyRemoteOpsBatch, getEditorContent};
use crate::hooks::use_core::pending;
use deve_core::protocol::ClientMessage;
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::*;

pub(super) fn handle_history(ctx: &SyncContext, ops: Vec<ConfirmedOp>) {
    ctx.set_pending_local_edits.update(|pending_edits| {
        let _ = pending::reconcile_with_history(pending_edits, ctx.doc_id, &ops);
    });
    ctx.set_history
        .set(merge_history_tail(&ops, ctx.history.get_untracked()));
    replay_pending_overlay(ctx);

    let txt = getEditorContent();
    emit_stats(ctx.on_stats, &txt);
    ctx.set_content.set(txt);
    ctx.set_playback_version
        .set(ctx.local_version.get_untracked());
    ctx.set_load_state.set("ready".to_string());
    ctx.set_load_progress.set((0, 0));
    ctx.set_load_eta_ms.set(0);
    resend_pending_edits(ctx);
}

fn merge_history_tail(
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

fn replay_pending_overlay(ctx: &SyncContext) {
    let ops = pending::cloned_ops_for_doc(&ctx.pending_local_edits.get_untracked(), ctx.doc_id);
    if ops.is_empty() {
        return;
    }
    if let Ok(json) = serde_json::to_string(&ops) {
        applyRemoteOpsBatch(&json);
    }
}

fn resend_pending_edits(ctx: &SyncContext) {
    let edits =
        pending::cloned_pending_edits_for_doc(&ctx.pending_local_edits.get_untracked(), ctx.doc_id);
    for edit in edits {
        ctx.ws.send(ClientMessage::Edit {
            doc_id: ctx.doc_id,
            op: edit.op,
            client_id: edit.client_id,
            client_op_id: edit.client_op_id,
        });
    }
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
