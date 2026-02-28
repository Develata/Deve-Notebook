// apps/cli/src/server/handlers/key_exchange.rs
//! # E2EE 密钥交换处理器
//!
//! 通过已认证的 WSS 通道向客户端提供 RepoKey。
//!
//! **安全模型**: TLS + JWT 双重保护。
//! **Invariant**: RepoKey 仅在内存中存在于客户端，页面卸载时清除。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 处理客户端的 RepoKey 请求
///
/// **Pre-condition**: 客户端已通过 JWT 认证 (middleware 保证)。
/// **Post-condition**: 成功时单播 `KeyProvide`，失败时单播 `KeyDenied`。
pub async fn handle_request_key(state: &Arc<AppState>, ch: &DualChannel) {
    match &state.repo_key {
        Some(key) => {
            tracing::info!("Providing RepoKey to authenticated client");
            ch.unicast(ServerMessage::KeyProvide {
                repo_key: key.to_bytes().to_vec(),
            });
        }
        None => {
            tracing::warn!("RepoKey requested but not configured");
            ch.unicast(ServerMessage::KeyDenied {
                reason: "Server has no RepoKey configured".into(),
            });
        }
    }
}
