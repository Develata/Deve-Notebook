//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
use super::context::SyncContext;
use super::history_replay::{
    apply_replay_ops_atomically, build_history_replay_ops, merge_history_tail,
};
use super::history_resend::resend_pending_edits_if_ready;
use crate::editor::EditorStats;
use crate::runtime::document::{confirm, pending};
use crate::runtime::domain::EditorSyncFailureCode;
use crate::runtime::domain::LoadPhase;
use crate::runtime::projection_recovery::RecoveryCompletion;
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::*;

pub(super) fn handle_history(ctx: &SyncContext, expected_generation: u64, ops: Vec<ConfirmedOp>) {
    if !ctx.is_generation_current(expected_generation) {
        return;
    }
    let mut next_pending = ctx.pending_local_edits.get_untracked();
    let _ = pending::reconcile_with_history(&mut next_pending, ctx.doc_id, &ops);
    let scope = pending::PendingScope::from_repo_id_str(
        ctx.current_repo_id.get_untracked().as_deref(),
        ctx.current_scope_nonce.get_untracked(),
    );
    let pending_ops = scope
        .map(|scope| pending::cloned_ops_for_doc_in_scope(&next_pending, ctx.doc_id, scope))
        .unwrap_or_default();
    let mut buffered = ctx.drain_buffered_live_ops();
    let base_version = ctx.local_version.get_untracked();
    let replay_ops = match build_history_replay_ops(
        &ops,
        &mut buffered,
        base_version,
        ctx.client_id,
        pending_ops,
    ) {
        Ok(replay_ops) => replay_ops,
        Err(code) => {
            ctx.restore_buffered_live_ops(buffered);
            ctx.fail_editor_sync(code);
            return;
        }
    };
    let Some(txt) =
        deve_core::state::try_apply_content_ops(&ctx.content.get_untracked(), &replay_ops)
    else {
        ctx.restore_buffered_live_ops(buffered);
        ctx.fail_editor_sync(EditorSyncFailureCode::HistoryReplay);
        return;
    };
    if let Err(code) = apply_replay_ops_atomically(&replay_ops) {
        ctx.restore_buffered_live_ops(buffered);
        ctx.fail_editor_sync(code);
        return;
    }
    buffered.dedup();

    let mut next_history = merge_history_tail(&ops, ctx.history.get_untracked());
    let mut next_version = next_history
        .last()
        .map(|(seq, _)| *seq)
        .unwrap_or(base_version)
        .max(base_version);
    let mut clear_navigation = false;
    for entry in buffered {
        if let Some(origin) = entry
            .origin
            .filter(|origin| Some(origin.client_id) == ctx.client_id)
            && let Some(scope) = scope
        {
            clear_navigation |= confirm::commit_pending_edit(
                &mut next_pending,
                Some(ctx.doc_id),
                Some(scope.repo_id),
                Some(scope.scope_nonce),
                ctx.doc_id,
                origin.client_op_id,
                entry.seq,
            )
            .clear_navigation;
        }
        if entry.seq > next_version {
            next_version = entry.seq;
            next_history.push((entry.seq, entry.op));
        }
    }

    ctx.set_pending_local_edits.set(next_pending);
    if clear_navigation {
        ctx.set_pending_navigation.set(None);
    }
    ctx.set_history.set(next_history);
    ctx.set_local_version.set(next_version);
    emit_stats(ctx.on_stats, &txt);
    ctx.set_content.set(txt);
    ctx.set_playback_version.set(next_version);
    if let RecoveryCompletion::ReopenTrailing(_) = ctx
        .projection_recovery
        .finish_generation(expected_generation)
    {
        super::recovery::reopen_active(ctx);
        return;
    }
    ctx.mark_live_ready(expected_generation);
    ctx.set_editor_sync_failure.set(None);
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
