// apps/web/src/editor/sync/mod.rs
//! # Sync Logic (同步逻辑)
//!
//! 处理来自 WebSocket 的 `ServerMessage`，分发至各子模块。
//! 拆分为 context (参数打包) / snapshot (快照处理) / decrypt (E2EE) 子模块。

pub mod context;
mod decrypt;
mod snapshot;

use super::EditorStats;
use super::ffi::{applyRemoteOp, getEditorContent};
use context::SyncContext;
use deve_core::protocol::ServerMessage;
use deve_core::security::RepoKey;
use leptos::prelude::*;

pub fn handle_server_message(msg: ServerMessage, ctx: &SyncContext) {
    match msg {
        ServerMessage::Snapshot {
            doc_id: msg_doc_id,
            content,
            base_seq,
            version,
            delta_ops,
        } => {
            if msg_doc_id != ctx.doc_id {
                return;
            }
            snapshot::handle_snapshot(ctx, content, base_seq, version, delta_ops);
        }
        ServerMessage::History {
            doc_id: msg_doc_id,
            ops,
        } => {
            if msg_doc_id != ctx.doc_id {
                return;
            }
            leptos::logging::log!("Received History: {} ops", ops.len());
            ctx.set_history.set(ops);
        }
        ServerMessage::NewOp {
            doc_id: msg_doc_id,
            op,
            seq,
            client_id: origin_id,
        } => {
            if msg_doc_id != ctx.doc_id {
                return;
            }
            handle_new_op(ctx, op, seq, origin_id);
        }
        ServerMessage::SyncHello {
            peer_id, vector: _, ..
        } => {
            leptos::logging::log!("P2P Handshake from Peer: {}", peer_id);
        }
        ServerMessage::Pong => {}
        ServerMessage::SyncPush { ops } => {
            decrypt::handle_sync_push(ctx, &ops);
        }
        ServerMessage::KeyProvide { repo_key } => {
            handle_key_provide(ctx, &repo_key);
        }
        ServerMessage::KeyDenied { reason } => {
            leptos::logging::warn!("KeyDenied: {}", reason);
        }
        _ => {}
    }
}

/// E2EE: 收到服务端提供的 RepoKey，存入内存信号
///
/// # Invariants
/// - `repo_key` 必须恰好 32 bytes (AES-256)
/// - 仅存于 RAM 信号中，页面卸载时自动清除
fn handle_key_provide(ctx: &SyncContext, raw: &[u8]) {
    match RepoKey::from_bytes(raw) {
        Some(key) => {
            leptos::logging::log!("E2EE: RepoKey received ({} bytes)", raw.len());
            ctx.set_repo_key.set(Some(key));
        }
        None => {
            leptos::logging::error!("E2EE: Invalid RepoKey length: {}", raw.len());
        }
    }
}

fn handle_new_op(ctx: &SyncContext, op: deve_core::models::Op, seq: u64, origin_id: u64) {
    let current_ver = ctx.local_version.get_untracked();
    if seq <= current_ver {
        return;
    }

    // 过滤回显 (Echoes)
    if origin_id != ctx.client_id {
        if let Ok(json) = serde_json::to_string(&op) {
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
    }
    ctx.set_local_version.set(seq);
    ctx.set_history.update(|h| h.push((seq, op)));

    // 如果处于 "head" (实时) 状态，自动推进回放
    if !ctx.is_playback.get_untracked() {
        ctx.set_playback_version.set(seq);
    }
}
