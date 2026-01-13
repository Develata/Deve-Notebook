use leptos::prelude::*;
use deve_core::models::{DocId, Op, LedgerEntry};
use super::ffi::applyRemoteContent;

pub fn handle_playback_change(
    ver: u64,
    doc_id: DocId,
    local_version: u64,
    history: ReadSignal<Vec<(u64, Op)>>,
    set_is_playback: WriteSignal<bool>,
) {
    // If ver < local, it's Playback mode.
    let is_pb = ver < local_version;
    set_is_playback.set(is_pb);

    // Only reconstruct when actually in playback mode
    // If viewing current version (is_pb == false), do NOT wipe content
    if !is_pb {
        return;
    }

    // Guard against empty history (nothing to reconstruct)
    let hist = history.get_untracked();
    if hist.is_empty() {
        return;
    }

    // Filter history <= ver
    let relevant_ops: Vec<LedgerEntry> = hist.into_iter()
        .filter(|(s, _)| *s <= ver)
        .map(|(_, op)| LedgerEntry {
             doc_id, op, timestamp: 0 // timestamp irrelevant for reconstruction
        })
        .collect();
        
    let reconstructed = deve_core::state::reconstruct_content(&relevant_ops);
    applyRemoteContent(&reconstructed);
    // We do NOT update `content` signal here to avoid triggering diffs loops.
    // CodeMirror update via applyRemoteContent is enough for visual.
}
