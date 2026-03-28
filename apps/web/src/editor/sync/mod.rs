// apps/web/src/editor/sync/mod.rs
//! 处理编辑器相关的同步消息。

pub mod context;
mod decrypt;
mod dispatch_doc;
mod dispatch_payload;
mod history;
mod key;
mod live;
mod scope;
mod snapshot;
mod snapshot_finish;

use context::SyncContext;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use leptos::prelude::GetUntracked;
use scope::{ScopedMessageScope, SyncPayloadScope, accepts_sync_payload};

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
        } => dispatch_doc::handle_snapshot_message(
            ctx,
            repo_id,
            branch,
            scope_nonce,
            msg_doc_id,
            request_id,
            content,
            base_seq,
            version,
            delta_ops,
        ),
        ServerMessage::History {
            repo_id,
            branch,
            scope_nonce,
            doc_id: msg_doc_id,
            request_id,
            ops,
        } => dispatch_doc::handle_history_message(
            ctx,
            repo_id,
            branch,
            scope_nonce,
            msg_doc_id,
            request_id,
            ops,
        ),
        ServerMessage::NewOp {
            repo_id,
            branch,
            scope_nonce,
            doc_id: msg_doc_id,
            entry,
        } => dispatch_doc::handle_new_op_message(
            ctx,
            repo_id,
            branch,
            scope_nonce,
            msg_doc_id,
            entry,
        ),
        ServerMessage::SyncHello {
            peer_id, vector: _, ..
        } => {
            let _ = peer_id;
        }
        ServerMessage::WriteReady {
            repo_id,
            scope_nonce,
            branch,
            ..
        } => dispatch_payload::handle_write_ready_message(ctx, repo_id, branch, scope_nonce),
        ServerMessage::Pong => {}
        ServerMessage::SyncPush {
            repo_id,
            scope_nonce,
            branch,
            ops,
        } => dispatch_payload::handle_sync_push_message(ctx, repo_id, branch, scope_nonce, &ops),
        ServerMessage::SyncPushSnapshot {
            repo_id,
            scope_nonce,
            branch,
            ops,
            ..
        } => dispatch_payload::handle_sync_push_message(ctx, repo_id, branch, scope_nonce, &ops),
        ServerMessage::KeyProvide {
            repo_id,
            scope_nonce,
            branch,
            repo_key,
        } => dispatch_payload::handle_key_provide_message(
            ctx,
            repo_id,
            branch,
            scope_nonce,
            &repo_key,
        ),
        ServerMessage::KeyDenied {
            repo_id,
            scope_nonce,
            branch,
            error,
        } => dispatch_payload::handle_key_denied_message(ctx, repo_id, branch, scope_nonce, &error),
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
