// apps\web\src\editor
//! # Playback Logic (回放逻辑)
//!
//! **架构作用**:
//! 处理历史版本回放的逻辑。
//! 当用户拖动时间轴时，从 `Op` 历史记录中重建文档内容。

use super::ffi::applyRemoteContent;
use deve_core::models::{DocId, LedgerEntry, Op};
use leptos::prelude::*;

pub fn handle_playback_change(
    ver: u64,
    doc_id: DocId,
    local_version: u64,
    history: ReadSignal<Vec<(u64, Op)>>,
    set_is_playback: WriteSignal<bool>,
) {
    // 如果 ver < local，则为回放模式。
    let is_pb = ver < local_version;
    set_is_playback.set(is_pb);

    // 仅在实际处于回放模式时重建
    // 如果查看当前版本 (is_pb == false)，不要擦除内容
    if !is_pb {
        return;
    }

    // 防止空历史记录 (无内容可重建)
    let hist = history.get_untracked();
    if hist.is_empty() {
        return;
    }

    // 过滤 history <= ver
    let relevant_ops: Vec<LedgerEntry> = hist
        .into_iter()
        .filter(|(s, _)| *s <= ver)
        .map(|(_, op)| LedgerEntry {
            doc_id,
            op,
            timestamp: 0, // 时间戳对于重建无关紧要
            peer_id: deve_core::models::PeerId::new("playback"),
            seq: 0,
        })
        .collect();

    let reconstructed = deve_core::state::reconstruct_content(&relevant_ops);
    applyRemoteContent(&reconstructed);
    // 我们在此不更新 `content` 信号，以避免触发 diff 循环。
    // 通过 applyRemoteContent 更新 CodeMirror 足以进行视觉展示。
}
