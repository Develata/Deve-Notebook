// apps/cli/src/server/handlers/key_exchange.rs
//! # E2EE 密钥交换处理器
//!
//! 通过已认证的 WSS 通道向客户端提供 RepoKey。
//!
//! **安全模型**: TLS + JWT 双重保护。
//! **Invariant**: RepoKey 仅在内存中存在于客户端，页面卸载时清除。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::resolve_session_repo;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

/// 处理客户端的 RepoKey 请求
///
/// **Pre-condition**: 客户端已通过 JWT 认证 (middleware 保证)。
/// **Post-condition**: 成功时单播 `KeyProvide`，失败时单播 `KeyDenied`。
pub async fn handle_request_key(state: &Arc<AppState>, ch: &DualChannel, session: &WsSession) {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            send_key_denied(ch, ServerErrorCode::SyncRepoUnbound, err.to_string());
            return;
        }
    };

    let key_dir = match state.repo.local_repo_notegit_keys_root(&scope.repo_name) {
        Ok(dir) => dir,
        Err(err) => {
            send_key_denied(ch, ServerErrorCode::StoragePersistFailed, err.to_string());
            return;
        }
    };

    match deve_core::security::load_or_generate_repo_key_at(&key_dir) {
        Ok(key) => {
            tracing::info!(
                "Providing RepoKey to authenticated client for {}",
                scope.repo_name
            );
            ch.unicast(ServerMessage::KeyProvide {
                repo_key: key.to_bytes().to_vec(),
            });
        }
        Err(err) => {
            tracing::warn!("RepoKey request failed for {}: {:?}", scope.repo_name, err);
            send_key_denied(ch, ServerErrorCode::StoragePersistFailed, err.to_string());
        }
    }
}

fn send_key_denied(ch: &DualChannel, code: ServerErrorCode, detail: impl Into<String>) {
    ch.unicast(ServerMessage::KeyDenied {
        error: ServerError::with_detail(code, detail),
    });
}
