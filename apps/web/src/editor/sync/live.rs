use super::context::SyncContext;
use crate::editor::EditorStats;
use crate::editor::ffi::{applyRemoteOp, getEditorContent};
use leptos::prelude::{Callable, GetUntracked, Set, Update};

pub fn handle_new_op(ctx: &SyncContext, entry: deve_core::protocol::ConfirmedOp) {
    if !ctx.is_live_ready() {
        ctx.buffer_live_op(entry);
        return;
    }
    apply_live_op(ctx, entry);
}

pub(super) fn apply_live_op(ctx: &SyncContext, entry: deve_core::protocol::ConfirmedOp) {
    if entry.seq <= ctx.local_version.get_untracked() {
        return;
    }
    let echoed = entry.origin.as_ref().map(|origin| origin.client_id) == ctx.client_id;
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
