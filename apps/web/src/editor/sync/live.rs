//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 10_rendering#document-authority-bridge
//!
use super::context::SyncContext;
use crate::editor::EditorStats;
use crate::editor::ffi::{applyRemoteOp, getEditorContent};
use crate::runtime::document::pending;
use deve_core::models::RepoId;
use leptos::prelude::{Callable, GetUntracked, Set, Update};

pub fn handle_new_op(ctx: &SyncContext, entry: deve_core::protocol::ConfirmedOp) {
    if !ctx.is_live_ready() {
        ctx.buffer_live_op(entry);
        return;
    }
    apply_live_op(ctx, entry);
}

pub(super) fn apply_live_op(ctx: &SyncContext, entry: deve_core::protocol::ConfirmedOp) {
    let echoed_origin = entry
        .origin
        .filter(|origin| Some(origin.client_id) == ctx.client_id);
    if let Some(origin) = echoed_origin {
        clear_confirmed_pending_edit(ctx, origin.client_op_id);
    }
    if entry.seq <= ctx.local_version.get_untracked() {
        return;
    }
    let echoed = echoed_origin.is_some();
    if !echoed {
        if let Ok(json) = serde_json::to_string(&entry.op) {
            applyRemoteOp(&json);
        }
        let text = getEditorContent();
        if let Some(cb) = ctx.on_stats {
            cb.run(EditorStats {
                chars: text.len(),
                words: text.split_whitespace().count(),
                lines: text.lines().count(),
            });
        }
        ctx.set_content.set(text);
    }
    ctx.set_local_version.set(entry.seq);
    ctx.set_history
        .update(|history| history.push((entry.seq, entry.op)));
    if !ctx.is_playback.get_untracked() {
        ctx.set_playback_version.set(entry.seq);
    }
}

fn clear_confirmed_pending_edit(ctx: &SyncContext, client_op_id: u64) {
    let Some(repo_id) = ctx
        .current_repo_id
        .get_untracked()
        .and_then(|repo_id| repo_id.parse::<RepoId>().ok())
    else {
        return;
    };
    let scope_nonce = Some(ctx.current_scope_nonce.get_untracked());
    let mut clear_navigation = false;
    ctx.set_pending_local_edits.update(|pending_edits| {
        clear_navigation = pending::clear_pending_edit_and_check_current_doc_empty(
            pending_edits,
            Some(ctx.doc_id),
            Some(repo_id),
            scope_nonce,
            ctx.doc_id,
            client_op_id,
        );
    });
    if clear_navigation {
        ctx.set_pending_navigation.set(None);
    }
}

#[cfg(test)]
mod tests;
