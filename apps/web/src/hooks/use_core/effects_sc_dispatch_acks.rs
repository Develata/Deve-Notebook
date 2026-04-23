use deve_core::protocol::ServerMessage;

use super::ScMessageContext;
use super::dispatch_ack_helpers::{
    handle_commit_ack, handle_conflict_resolved_ack, handle_fs_change_ack, handle_simple_ack,
};

pub(crate) fn handle_sc_ack_message(
    msg: &ServerMessage,
    ctx: &ScMessageContext<'_>,
    active_scope_nonce: u64,
) -> bool {
    match msg {
        ServerMessage::StageAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            handle_simple_ack(ctx, "已暂存", "Staged", path);
            true
        }
        ServerMessage::UnstageAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            handle_simple_ack(ctx, "已取消暂存", "Unstaged", path);
            true
        }
        ServerMessage::DiscardAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            handle_simple_ack(ctx, "已放弃变更", "Discarded", path);
            true
        }
        ServerMessage::CommitAck {
            commit_id,
            repo_id,
            branch,
            scope_nonce,
            ..
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            handle_commit_ack(ctx, commit_id, active_scope_nonce);
            true
        }
        ServerMessage::FsChangeDetected {
            repo_id,
            branch,
            scope_nonce,
            path,
            change_type,
            has_conflict,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            handle_fs_change_ack(ctx, active_scope_nonce, path, change_type, *has_conflict);
            true
        }
        ServerMessage::ConflictResolved {
            repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            handle_conflict_resolved_ack(ctx, path, resolution);
            true
        }
        _ => false,
    }
}
