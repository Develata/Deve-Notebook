//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 05_network#web-ws-runtime
//!
use super::SyncContext;
use super::dispatch_payload;
use deve_core::protocol::ServerMessage;

pub(super) fn route_payload_message(
    msg: ServerMessage,
    ctx: &SyncContext,
) -> Option<ServerMessage> {
    match msg {
        ServerMessage::WriteReady {
            repo_id,
            scope_nonce,
            branch,
            ..
        } => {
            dispatch_payload::handle_write_ready_message(ctx, repo_id, branch, scope_nonce.get());
            None
        }
        ServerMessage::Pong => None,
        ServerMessage::SyncPush {
            repo_id,
            scope_nonce,
            branch,
            encrypted_payload,
            ..
        } => {
            dispatch_payload::handle_sync_push_message(
                ctx,
                repo_id,
                branch,
                scope_nonce.get(),
                &encrypted_payload,
            );
            None
        }
        ServerMessage::SyncPushSnapshot {
            repo_id,
            scope_nonce,
            branch,
            payload,
            ..
        } => {
            dispatch_payload::handle_sync_push_message(
                ctx,
                repo_id,
                branch,
                scope_nonce.get(),
                &payload,
            );
            None
        }
        ServerMessage::KeyProvide {
            repo_id,
            scope_nonce,
            branch,
            repo_key,
        } => {
            dispatch_payload::handle_key_provide_message(
                ctx,
                repo_id,
                branch,
                scope_nonce.get(),
                &repo_key,
            );
            None
        }
        ServerMessage::KeyDenied {
            repo_id,
            scope_nonce,
            branch,
            error,
        } => {
            dispatch_payload::handle_key_denied_message(
                ctx,
                repo_id,
                branch,
                scope_nonce.get(),
                &error,
            );
            None
        }
        other => Some(other),
    }
}
