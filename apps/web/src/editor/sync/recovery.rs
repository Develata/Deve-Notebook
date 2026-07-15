//! plan_ref:
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#projection-recovery-coordinator
//!   - 10_rendering#document-authority-bridge
//!
//! Editor adapter for backend-owned projection recovery plans.

use super::context::SyncContext;
use crate::editor::buffered_ops::clear_sync_buffers;
use crate::editor::hook_open::advance_session_generation;
use crate::runtime::domain::LoadPhase;
use crate::runtime::projection_recovery::{
    ProjectionRecoveryScope, RecoveryStart, evaluate_recovery,
};
use deve_core::protocol::{ClientMessage, ProjectionRecoveryRequired};
use leptos::prelude::{GetUntracked, Set};
use std::sync::atomic::Ordering;

pub(super) fn handle_required(ctx: &SyncContext, required: ProjectionRecoveryRequired) {
    if ctx.current_doc.get_untracked() != Some(ctx.doc_id) {
        return;
    }
    let current_repo_id = ctx
        .current_repo_id
        .get_untracked()
        .and_then(|repo_id| repo_id.parse().ok());
    let decision = evaluate_recovery(
        &required,
        &ProjectionRecoveryScope {
            repo_id: current_repo_id,
            branch: ctx.active_branch.get_untracked(),
            scope_nonce: ctx.current_scope_nonce.get_untracked(),
            current_doc: ctx.current_doc.get_untracked(),
            scope_switch_pending: ctx.pending_repo_switch.get_untracked().is_some()
                || ctx.pending_branch_switch.get_untracked().is_some(),
        },
    );
    if !decision.is_some_and(|decision| decision.current_document_affected) {
        return;
    }

    if ctx.projection_recovery.begin(required) == RecoveryStart::ReopenNow {
        reopen_active(ctx);
    }
}

pub(super) fn reopen_active(ctx: &SyncContext) {
    let request_id = advance_session_generation(&ctx.session_generation);
    ctx.projection_recovery.mark_generation(request_id);
    ctx.ready_generation.store(0, Ordering::Relaxed);
    clear_sync_buffers(
        &ctx.buffered_live_ops,
        &ctx.buffered_encrypted_ops,
        "projection recovery clears confirmed live buffer",
        "projection recovery clears encrypted live buffer",
    );
    lock_editor_projection();
    ctx.set_open_request_id.set(request_id);
    ctx.set_local_version.set(0);
    ctx.set_history.set(Vec::new());
    ctx.set_playback_version.set(0);
    ctx.set_editor_sync_failure.set(None);
    ctx.set_snapshot_reopen_attempted.set(false);
    ctx.set_load_state.set(LoadPhase::Resyncing);
    ctx.set_load_progress.set((0, 0));
    ctx.set_load_eta_ms.set(0);
    ctx.ws.send(ClientMessage::OpenDoc {
        doc_id: ctx.doc_id,
        request_id,
        scope_nonce: Some(ctx.current_scope_nonce.get_untracked()),
    });
}

#[cfg(target_arch = "wasm32")]
fn lock_editor_projection() {
    crate::editor::ffi::set_read_only(true);
}

#[cfg(not(target_arch = "wasm32"))]
fn lock_editor_projection() {}

#[cfg(test)]
mod tests;
