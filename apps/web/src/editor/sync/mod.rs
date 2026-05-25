// apps/web/src/editor/sync/mod.rs
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 10_rendering#document-authority-bridge
//!
//! 处理编辑器相关的同步消息。

pub mod context;
mod decrypt;
mod dispatch_doc;
mod dispatch_payload;
mod history;
mod history_replay;
mod history_resend;
mod key;
mod live;
mod route_doc;
mod route_payload;
mod scope;
mod snapshot;
mod snapshot_apply;
mod snapshot_finish;
mod snapshot_gate;

use context::SyncContext;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use leptos::prelude::GetUntracked;
use route_doc::route_doc_message;
use route_payload::route_payload_message;
use scope::{ScopedMessageScope, SyncPayloadScope, accepts_sync_payload};

pub fn handle_server_message(msg: ServerMessage, ctx: &SyncContext) {
    let Some(msg) = route_doc_message(msg, ctx) else {
        return;
    };
    let Some(msg) = route_payload_message(msg, ctx) else {
        return;
    };
    if let ServerMessage::SyncHello {
        peer_id, vector: _, ..
    } = msg
    {
        let _ = peer_id;
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
