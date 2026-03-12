// apps/web/src/editor/sync/mod.rs
//! 处理编辑器相关的同步消息。

pub mod context;
mod decrypt;
mod history;
mod scope;
mod snapshot;
mod snapshot_finish;

use super::EditorStats;
use super::ffi::{applyRemoteOp, getEditorContent};
use context::SyncContext;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use deve_core::security::RepoKey;
use leptos::prelude::*;
use scope::matches_scope;

pub fn handle_server_message(msg: ServerMessage, ctx: &SyncContext) {
    match msg {
        ServerMessage::Snapshot {
            repo_id,
            branch,
            doc_id: msg_doc_id,
            request_id,
            content,
            base_seq,
            version,
            delta_ops,
        } => {
            if !matches_current_scope(ctx, Some(repo_id), branch) {
                return;
            }
            if msg_doc_id != ctx.doc_id {
                return;
            }
            if request_id != ctx.open_request_id.get_untracked() {
                return;
            }
            snapshot::handle_snapshot(ctx, request_id, content, base_seq, version, delta_ops);
        }
        ServerMessage::History {
            repo_id,
            branch,
            doc_id: msg_doc_id,
            request_id,
            ops,
        } => {
            if !matches_current_scope(ctx, Some(repo_id), branch) {
                return;
            }
            if msg_doc_id != ctx.doc_id || request_id != ctx.open_request_id.get_untracked() {
                return;
            }
            leptos::logging::log!("Received History: {} ops", ops.len());
            history::handle_history(ctx, ops);
        }
        ServerMessage::NewOp {
            repo_id,
            branch,
            doc_id: msg_doc_id,
            entry,
        } => {
            if !matches_current_scope(ctx, Some(repo_id), branch) {
                return;
            }
            if msg_doc_id != ctx.doc_id {
                return;
            }
            handle_new_op(ctx, entry);
        }
        ServerMessage::SyncHello {
            peer_id, vector: _, ..
        } => {
            leptos::logging::log!("P2P Handshake from Peer: {}", peer_id);
        }
        ServerMessage::Pong => {}
        ServerMessage::SyncPush {
            repo_id,
            branch,
            ops,
        } => {
            if matches_current_scope(ctx, Some(repo_id), branch) {
                decrypt::handle_sync_push(ctx, &ops);
            }
        }
        ServerMessage::SyncPushSnapshot {
            repo_id,
            branch,
            ops,
            ..
        } => {
            if matches_current_scope(ctx, Some(repo_id), branch) {
                decrypt::handle_sync_push(ctx, &ops);
            }
        }
        ServerMessage::KeyProvide {
            repo_id,
            branch,
            repo_key,
        } => {
            if matches_current_scope(ctx, Some(repo_id), branch) {
                handle_key_provide(ctx, &repo_key);
            }
        }
        ServerMessage::KeyDenied {
            repo_id,
            branch,
            error,
        } => {
            if matches_current_scope(ctx, repo_id, branch) {
                ctx.set_repo_key.set(None);
                leptos::logging::warn!(
                    "KeyDenied: code={:?} detail={:?}",
                    error.code,
                    error.detail
                );
            }
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

fn matches_current_scope(
    ctx: &SyncContext,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
) -> bool {
    matches_scope(
        ctx.current_repo_id.get_untracked(),
        ctx.pending_repo_switch.get_untracked(),
        ctx.active_branch.get_untracked(),
        ctx.pending_branch_switch.get_untracked(),
        repo_id,
        branch,
    )
}

fn handle_new_op(ctx: &SyncContext, entry: deve_core::protocol::ConfirmedOp) {
    if entry.seq <= ctx.local_version.get_untracked() {
        return;
    }
    let echoed = entry.origin.as_ref().map(|origin| origin.client_id) == ctx.client_id;
    if !echoed {
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
    }
    ctx.set_local_version.set(entry.seq);
    ctx.set_history.update(|h| h.push((entry.seq, entry.op)));
    if !ctx.is_playback.get_untracked() {
        ctx.set_playback_version.set(entry.seq);
    }
}
