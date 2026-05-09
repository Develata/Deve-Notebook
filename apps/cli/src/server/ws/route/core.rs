//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Core WebSocket message routing.

use crate::server::handlers::{plugin, switcher, sync};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::{ClientMessage, ServerMessage};
use std::sync::Arc;

/// 路由剩余的核心消息（内容、查询、切换、快照同步等）。
pub(super) async fn route_core(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    if let Some(msg) = super::core_scoped::route_scoped_core(state, ch, session, msg).await {
        route_unscoped_core(state, ch, session, msg).await;
    }
}

async fn route_unscoped_core(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::PluginCall {
            req_id,
            plugin_id,
            fn_name,
            args,
        } => {
            plugin::handle_plugin_call(state, ch, req_id, plugin_id, fn_name, args).await;
        }
        ClientMessage::SwitchBranch {
            peer_id,
            switch_nonce,
        } => {
            switcher::handle_switch_branch(state, ch, session, peer_id, switch_nonce).await;
        }
        ClientMessage::SwitchRepo { name, switch_nonce } => {
            switcher::handle_switch_repo(state, ch, session, name, None, switch_nonce).await;
        }
        ClientMessage::SwitchRepoExact {
            name,
            repo_id,
            switch_nonce,
        } => {
            switcher::handle_switch_repo(state, ch, session, name, Some(repo_id), switch_nonce)
                .await;
        }
        ClientMessage::SyncSnapshotRequest {
            source_peer_id,
            repo_id,
            reason,
            ..
        } => {
            sync::handle_sync_snapshot_request(state, ch, session, source_peer_id, repo_id, reason)
                .await;
        }
        ClientMessage::SyncPushSnapshot {
            source_peer_id,
            repo_id,
            payload,
            ..
        } => {
            sync::handle_sync_push_snapshot(state, ch, session, source_peer_id, repo_id, payload)
                .await;
        }
        ClientMessage::SyncRequest {
            repo_id, requests, ..
        } => {
            sync::handle_sync_request(state, ch, session, repo_id, requests).await;
        }
        ClientMessage::SyncPush {
            source_peer_id,
            repo_id,
            header,
            encrypted_payload,
        } => {
            sync::handle_sync_push(
                state,
                ch,
                session,
                source_peer_id,
                repo_id,
                header,
                encrypted_payload,
            )
            .await;
        }
        ClientMessage::RegisterWriter {
            peer_id,
            repo_id,
            scope_nonce,
        } => {
            sync::handle_register_writer(ch, session, repo_id, peer_id, scope_nonce.get());
        }
        ClientMessage::Ping => {
            ch.unicast(ServerMessage::Pong);
        }
        other => {
            tracing::debug!("Unhandled client message: {:?}", other);
        }
    }
}

#[cfg(test)]
mod tests;
