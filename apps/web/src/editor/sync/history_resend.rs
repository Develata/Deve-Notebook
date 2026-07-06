//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use super::context::SyncContext;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::runtime::document::pending;
use crate::runtime::scope_client::{LocalScopeSignals, stable_local_scope_nonce};
use deve_core::protocol::ClientMessage;
use leptos::prelude::GetUntracked;

pub(super) fn resend_pending_edits_if_ready(ctx: &SyncContext) {
    if repo_write_block_untracked(
        ctx.ws,
        RepoWriteSignals {
            load_state: ctx.load_state,
            is_spectator: ctx.is_spectator,
            handshake_ready: ctx.handshake_ready,
            current_repo_id: ctx.current_repo_id,
            current_scope_nonce: ctx.current_scope_nonce,
            active_branch: ctx.active_branch,
            pending_branch_switch: ctx.pending_branch_switch,
            pending_repo_switch: ctx.pending_repo_switch,
        },
    )
    .is_some()
    {
        return;
    }
    resend_pending_edits(ctx);
}

fn resend_pending_edits(ctx: &SyncContext) {
    let Some(scope_nonce) = stable_local_scope_nonce(LocalScopeSignals {
        current_repo_id: ctx.current_repo_id,
        current_scope_nonce: ctx.current_scope_nonce,
        active_branch: ctx.active_branch,
        pending_branch_switch: ctx.pending_branch_switch,
        pending_repo_switch: ctx.pending_repo_switch,
    }) else {
        return;
    };
    let Some(scope) = pending::PendingScope::from_repo_id_str(
        ctx.current_repo_id.get_untracked().as_deref(),
        scope_nonce,
    ) else {
        return;
    };
    let edits = pending::cloned_pending_edits_for_doc_in_scope(
        &ctx.pending_local_edits.get_untracked(),
        ctx.doc_id,
        scope,
    );
    for edit in edits {
        ctx.ws.send(ClientMessage::Edit {
            doc_id: ctx.doc_id,
            op: edit.op,
            client_id: edit.client_id,
            client_op_id: edit.client_op_id,
            scope_nonce: Some(edit.scope_nonce),
        });
    }
}
