// apps/web/src/editor/sync/decrypt.rs
//! # E2EE Decrypt (客户端解密)
//!
//! 处理来自 P2P 同步的加密操作，使用 RepoKey 解密后应用到编辑器。
//!
//! ## Invariants
//! - 若无 RepoKey，加密操作将被跳过并记录警告
//! - 解密后的 LedgerEntry.op 与 NewOp 走相同的应用路径

use super::context::SyncContext;
use crate::editor::EditorStats;
use crate::editor::ffi::{applyRemoteOp, getEditorContent};
use deve_core::security::EncryptedOp;
use leptos::prelude::*;

/// 解密并应用 P2P 同步推送的加密操作
///
/// # Pre-conditions
/// - `ctx.repo_key` 已通过 KeyProvide 设置 (否则跳过)
/// - `ops` 中的 EncryptedOp 使用与 repo_key 相同的 AES-256 密钥加密
///
/// # Post-conditions
/// - 成功解密的 op 被应用到编辑器 (与 handle_new_op 相同路径)
/// - 失败的 op 被跳过并记录错误
pub fn handle_sync_push(ctx: &SyncContext, ops: &[EncryptedOp]) {
    let key = ctx.repo_key.get_untracked();
    let Some(key) = key else {
        leptos::logging::warn!("SyncPush: {} encrypted ops skipped (no RepoKey)", ops.len());
        return;
    };

    leptos::logging::log!("SyncPush: decrypting {} ops", ops.len());

    for enc_op in ops {
        match key.decrypt(enc_op) {
            Ok(entry) => apply_decrypted_entry(ctx, entry, enc_op.seq),
            Err(e) => {
                leptos::logging::error!("Decrypt failed seq={}: {}", enc_op.seq, e);
            }
        }
    }
}

/// 将解密后的 LedgerEntry 应用到编辑器
///
/// 逻辑与 handle_new_op 对齐：过滤回显、更新版本、推进回放。
fn apply_decrypted_entry(ctx: &SyncContext, entry: deve_core::models::LedgerEntry, seq: u64) {
    let current_ver = ctx.local_version.get_untracked();
    if seq <= current_ver {
        return;
    }

    if entry.doc_id != ctx.doc_id {
        return;
    }

    // 应用远程操作到编辑器
    if let Ok(json) = serde_json::to_string(&entry.op) {
        applyRemoteOp(&json);
    }

    let txt = getEditorContent();
    if let Some(cb) = ctx.on_stats {
        cb.run(EditorStats {
            chars: txt.len(),
            words: txt.split_whitespace().count(),
            lines: txt.lines().count(),
        });
    }
    ctx.set_content.set(txt);
    ctx.set_local_version.set(seq);
    ctx.set_history.update(|h| h.push((seq, entry.op)));

    if !ctx.is_playback.get_untracked() {
        ctx.set_playback_version.set(seq);
    }
}
