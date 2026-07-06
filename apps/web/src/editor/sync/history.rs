//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
use super::context::SyncContext;
use super::history_replay::{merge_history_tail, replay_buffered_live_ops, replay_pending_overlay};
use super::history_resend::resend_pending_edits_if_ready;
use crate::editor::EditorStats;
use crate::editor::ffi::getEditorContent;
use crate::runtime::document::pending;
use crate::runtime::domain::LoadPhase;
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::*;

pub(super) fn handle_history(ctx: &SyncContext, expected_generation: u64, ops: Vec<ConfirmedOp>) {
    if !ctx.is_generation_current(expected_generation) {
        return;
    }
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
    ctx.mark_live_ready(expected_generation);
    replay_buffered_live_ops(ctx, expected_generation);
    ctx.set_load_state.set(LoadPhase::Ready);
    ctx.set_load_progress.set((0, 0));
    ctx.set_load_eta_ms.set(0);
    resend_pending_edits_if_ready(ctx);
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
