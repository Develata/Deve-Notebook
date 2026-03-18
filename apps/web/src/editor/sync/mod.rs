// apps/web/src/editor/sync/mod.rs
//! 处理编辑器相关的同步消息。

pub mod context;
mod decrypt;
mod history;
mod key;
mod scope;
mod snapshot;
mod snapshot_finish;

use super::EditorStats;
use super::ffi::{applyRemoteOp, getEditorContent};
use context::SyncContext;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use key::handle_key_provide;
use leptos::prelude::*;
use scope::{ScopedMessageScope, SyncPayloadScope, accepts_sync_payload, matches_scoped_message};

pub fn handle_server_message(msg: ServerMessage, ctx: &SyncContext) {
    match msg {
        ServerMessage::Snapshot {
            repo_id,
            branch,
            scope_nonce,
            doc_id: msg_doc_id,
            request_id,
            content,
            base_seq,
            version,
            delta_ops,
        } => {
            if !matches_scoped_message(
                current_scoped_message_scope(ctx),
                Some(repo_id),
                branch.clone(),
                scope_nonce,
            ) {
                return;
            }
            if msg_doc_id != ctx.doc_id {
                return;
            }
            if request_id != ctx.open_request_id.get_untracked() {
                return;
            }
            let expected_generation = ctx.current_generation();
            snapshot::handle_snapshot(
                ctx,
                snapshot::SnapshotMessage {
                    expected_generation,
                    repo_id,
                    branch,
                    request_id,
                    new_content: content,
                    base_seq,
                    version,
                    delta_ops,
                },
            );
        }
        ServerMessage::History {
            repo_id,
            branch,
            scope_nonce,
            doc_id: msg_doc_id,
            request_id,
            ops,
        } => {
            if !matches_scoped_message(
                current_scoped_message_scope(ctx),
                Some(repo_id),
                branch,
                scope_nonce,
            ) {
                return;
            }
            if msg_doc_id != ctx.doc_id || request_id != ctx.open_request_id.get_untracked() {
                return;
            }
            let expected_generation = ctx.current_generation();
            leptos::logging::log!("Received History: {} ops", ops.len());
            history::handle_history(ctx, expected_generation, ops);
        }
        ServerMessage::NewOp {
            repo_id,
            branch,
            scope_nonce,
            doc_id: msg_doc_id,
            entry,
        } => {
            if !matches_scoped_message(
                current_scoped_message_scope(ctx),
                Some(repo_id),
                branch,
                scope_nonce,
            ) {
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
        ServerMessage::WriteReady {
            repo_id,
            scope_nonce,
            branch,
            ..
        } => {
            if accepts_current_sync_payload(ctx, repo_id, branch, scope_nonce) {
                history::resend_pending_edits(ctx);
            }
        }
        ServerMessage::Pong => {}
        ServerMessage::SyncPush {
            repo_id,
            scope_nonce,
            branch,
            ops,
        } => {
            if accepts_current_sync_payload(ctx, repo_id, branch, scope_nonce) {
                decrypt::handle_sync_push(ctx, &ops);
            }
        }
        ServerMessage::SyncPushSnapshot {
            repo_id,
            scope_nonce,
            branch,
            ops,
            ..
        } => {
            if accepts_current_sync_payload(ctx, repo_id, branch, scope_nonce) {
                decrypt::handle_sync_push(ctx, &ops);
            }
        }
        ServerMessage::KeyProvide {
            repo_id,
            scope_nonce,
            branch,
            repo_key,
        } => {
            if matches_scoped_message(
                current_scoped_message_scope(ctx),
                Some(repo_id),
                branch,
                Some(scope_nonce),
            ) {
                handle_key_provide(ctx, &repo_key);
            }
        }
        ServerMessage::KeyDenied {
            repo_id,
            scope_nonce,
            branch,
            error,
        } => {
            if matches_scoped_message(
                current_scoped_message_scope(ctx),
                repo_id,
                branch,
                Some(scope_nonce),
            ) {
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

fn current_scoped_message_scope(ctx: &SyncContext) -> ScopedMessageScope {
    ScopedMessageScope {
        current_repo_id: ctx.current_repo_id.get_untracked(),
        pending_repo_switch: ctx.pending_repo_switch.get_untracked(),
        current_branch: ctx.active_branch.get_untracked(),
        pending_branch_switch: ctx.pending_branch_switch.get_untracked(),
        current_scope_nonce: ctx.current_scope_nonce.get_untracked(),
    }
}

fn accepts_current_sync_payload(
    ctx: &SyncContext,
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: u64,
) -> bool {
    accepts_sync_payload(
        SyncPayloadScope {
            current_repo_id: ctx.current_repo_id.get_untracked(),
            pending_repo_switch: ctx.pending_repo_switch.get_untracked(),
            current_branch: ctx.active_branch.get_untracked(),
            pending_branch_switch: ctx.pending_branch_switch.get_untracked(),
            handshake_scope_nonce: ctx.handshake_scope_nonce.get_untracked(),
        },
        repo_id,
        branch,
        scope_nonce,
    )
}

fn handle_new_op(ctx: &SyncContext, entry: deve_core::protocol::ConfirmedOp) {
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
