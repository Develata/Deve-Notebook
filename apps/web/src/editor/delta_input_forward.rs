use super::delta_input::DeltaInputCtx;
use super::ffi::Delta;
use super::op_id::next_client_op_id;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::pending;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Update};

pub(super) fn forward_deltas(ctx: &DeltaInputCtx, deltas: Vec<Delta>) -> bool {
    let Some(client_id) = ctx
        .ws
        .writer_client_id_for(ctx.current_repo_id.get_untracked().as_deref())
    else {
        leptos::logging::warn!("Delta ignored: writer client id unavailable.");
        return false;
    };
    let Some(scope_nonce) = stable_local_scope_nonce(LocalScopeSignals {
        current_repo_id: ctx.current_repo_id,
        current_scope_nonce: ctx.current_scope_nonce,
        active_branch: ctx.active_branch,
        pending_branch_switch: ctx.pending_branch_switch,
        pending_repo_switch: ctx.pending_repo_switch,
    }) else {
        leptos::logging::warn!("Delta ignored: local scope nonce unavailable.");
        return false;
    };
    for delta in deltas {
        for op in delta.to_ops() {
            let client_op_id = next_client_op_id();
            ctx.set_pending_local_edits.update(|pending_edits| {
                pending::push_pending_edit(
                    pending_edits,
                    ctx.doc_id,
                    client_id,
                    client_op_id,
                    ctx.local_version.get_untracked(),
                    op.clone(),
                );
            });
            ctx.ws.send(ClientMessage::Edit {
                doc_id: ctx.doc_id,
                op: op.clone(),
                client_id,
                client_op_id,
                scope_nonce: Some(scope_nonce),
            });
        }
    }
    true
}
