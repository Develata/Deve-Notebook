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
            dispatch_payload::handle_write_ready_message(ctx, repo_id, branch, scope_nonce);
            None
        }
        ServerMessage::Pong => None,
        ServerMessage::SyncPush {
            repo_id,
            scope_nonce,
            branch,
            ops,
        } => {
            dispatch_payload::handle_sync_push_message(ctx, repo_id, branch, scope_nonce, &ops);
            None
        }
        ServerMessage::SyncPushSnapshot {
            repo_id,
            scope_nonce,
            branch,
            ops,
            ..
        } => {
            dispatch_payload::handle_sync_push_message(ctx, repo_id, branch, scope_nonce, &ops);
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
                scope_nonce,
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
            dispatch_payload::handle_key_denied_message(ctx, repo_id, branch, scope_nonce, &error);
            None
        }
        other => Some(other),
    }
}
