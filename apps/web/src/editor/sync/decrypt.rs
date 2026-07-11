// apps/web/src/editor/sync/decrypt.rs
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 10_rendering#document-authority-bridge
//!
//! # E2EE Decrypt (客户端解密)
//!
//! 处理来自 P2P 同步的加密操作，使用 RepoKey 解密后应用到编辑器。
//!
//! ## Invariants
//! - 若无 RepoKey，加密操作必须先进入缓冲，绝不能静默丢弃
//! - 解密后的内容事件与 `NewOp` 走相同的应用路径

use super::context::SyncContext;
use super::live::apply_live_op;
use deve_core::protocol::{ClientOrigin, ConfirmedOp};
use deve_core::security::{EncryptedOp, RepoKey};
use leptos::prelude::GetUntracked;
use std::sync::Mutex;

/// 解密并应用 P2P 同步推送的加密操作
///
/// # Pre-conditions
/// - 若 `ctx.repo_key` 尚未到达，则先缓存 `ops`
/// - `ops` 中的 EncryptedOp 使用与 repo_key 相同的 AES-256 密钥加密
///
/// # Post-conditions
/// - 无 key 时，`ops` 被缓冲等待 `KeyProvide`
/// - 有 key 时，成功解密的 op 被应用到编辑器 (与 handle_new_op 相同路径)
/// - 失败的 op 被跳过并记录错误
pub fn handle_sync_push(ctx: &SyncContext, ops: &[EncryptedOp]) {
    let Some(key) = buffer_encrypted_ops_until_key(
        &ctx.buffered_encrypted_ops,
        ctx.repo_key.get_untracked(),
        ops,
    ) else {
        return;
    };

    handle_sync_push_with_key(ctx, &key, ops);
}

pub(super) fn handle_sync_push_with_key(ctx: &SyncContext, key: &RepoKey, ops: &[EncryptedOp]) {
    leptos::logging::log!("SyncPush: decrypting {} ops", ops.len());

    for enc_op in ops {
        match key.decrypt(enc_op) {
            Ok(entry) => apply_decrypted_entry(ctx, entry, enc_op.peer_seq.get()),
            Err(e) => {
                leptos::logging::error!("Decrypt failed seq={}: {}", enc_op.peer_seq, e);
            }
        }
    }
}

pub(super) fn buffer_encrypted_ops_until_key(
    buffered: &Mutex<Vec<EncryptedOp>>,
    key: Option<RepoKey>,
    ops: &[EncryptedOp],
) -> Option<RepoKey> {
    let Some(key) = key else {
        leptos::logging::warn!(
            "SyncPush: buffering {} encrypted ops (no RepoKey)",
            ops.len()
        );
        match buffered.lock() {
            Ok(mut pending) => pending.extend_from_slice(ops),
            Err(_) => {
                leptos::logging::warn!("忽略 encrypted sync push: buffered_encrypted_ops 锁已损坏")
            }
        }
        return None;
    };
    Some(key)
}

#[cfg(test)]
mod tests;

/// 将解密后的 LedgerEntry 应用到编辑器
///
/// 逻辑与 handle_new_op 对齐：过滤回显、更新版本、推进回放。
fn apply_decrypted_entry(ctx: &SyncContext, entry: deve_core::models::LedgerEntry, seq: u64) {
    if entry.doc_id != Some(ctx.doc_id) {
        return;
    }
    let Some(op) = entry.cloned_content_op() else {
        return;
    };
    let origin = match (entry.client_id, entry.client_op_id) {
        (Some(client_id), Some(client_op_id)) => Some(ClientOrigin {
            client_id,
            client_op_id,
        }),
        _ => None,
    };
    let confirmed = ConfirmedOp::new(seq, op, origin);
    if ctx.is_live_ready() {
        apply_live_op(ctx, confirmed);
    } else {
        ctx.buffer_live_op(confirmed);
    }
}
