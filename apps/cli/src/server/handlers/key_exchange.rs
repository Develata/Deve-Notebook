// apps/cli/src/server/handlers/key_exchange.rs
//! # E2EE 密钥交换处理器
//!
//! 通过已认证的 WSS 通道向客户端提供 RepoKey。
//!
//! **安全模型**: TLS + JWT 双重保护。
//! **Invariant**: RepoKey 仅在内存中存在于客户端，页面卸载时清除。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    map_repo_scope_error, resolve_local_counterpart_repo, resolve_session_repo_and_sync,
};
use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

/// 处理客户端的 RepoKey 请求
///
/// **Pre-condition**: 客户端已通过 JWT 认证 (middleware 保证)。
/// **Post-condition**: 成功时单播 `KeyProvide`，失败时单播 `KeyDenied`。
pub async fn handle_request_key(state: &Arc<AppState>, ch: &DualChannel, session: &mut WsSession) {
    let Some(scope_nonce) = browser_scope_nonce(session) else {
        ch.send_protocol_error_with_scope_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                "request key is only supported for browser sessions",
            ),
            None,
        );
        return;
    };
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            send_key_denied_error(
                ch,
                scope_nonce,
                session.active_repo_id.or(session.bound_repo_id),
                session.active_branch.clone(),
                map_repo_scope_error(err),
            );
            return;
        }
    };
    let key_scope = match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => local_scope,
        Ok(None) => {
            send_key_denied_error(
                ch,
                scope_nonce,
                Some(scope.repo_id),
                session.active_branch.clone(),
                ServerError::with_detail(
                    ServerErrorCode::ScRepoContextInvalid,
                    "No local writable repo available for current scope",
                ),
            );
            return;
        }
        Err(err) => {
            send_key_denied_error(
                ch,
                scope_nonce,
                Some(scope.repo_id),
                session.active_branch.clone(),
                map_repo_scope_error(err),
            );
            return;
        }
    };

    let key_dir = match state
        .repo
        .local_repo_notegit_keys_root(&key_scope.repo_name)
    {
        Ok(dir) => dir,
        Err(err) => {
            send_key_denied_error(
                ch,
                scope_nonce,
                Some(scope.repo_id),
                session.active_branch.clone(),
                ServerError::with_detail(ServerErrorCode::StoragePersistFailed, err.to_string()),
            );
            return;
        }
    };

    match deve_core::security::load_or_generate_repo_key_at(&key_dir) {
        Ok(key) => {
            tracing::info!(
                "Providing RepoKey to authenticated client for {}",
                key_scope.repo_name
            );
            ch.unicast(ServerMessage::KeyProvide {
                repo_id: scope.repo_id,
                scope_nonce,
                branch: session.active_branch.clone(),
                repo_key: key.to_bytes().to_vec(),
            });
        }
        Err(err) => {
            tracing::warn!(
                "RepoKey request failed for {}: {:?}",
                key_scope.repo_name,
                err
            );
            send_key_denied_error(
                ch,
                scope_nonce,
                Some(scope.repo_id),
                session.active_branch.clone(),
                ServerError::with_detail(ServerErrorCode::StoragePersistFailed, err.to_string()),
            );
        }
    }
}

fn send_key_denied_error(
    ch: &DualChannel,
    scope_nonce: u64,
    repo_id: Option<deve_core::models::RepoId>,
    branch: Option<deve_core::models::PeerId>,
    error: ServerError,
) {
    ch.unicast(ServerMessage::KeyDenied {
        repo_id,
        scope_nonce,
        branch,
        error,
    });
}

fn browser_scope_nonce(session: &WsSession) -> Option<u64> {
    session.is_browser_session().then(|| session.scope_nonce())
}
