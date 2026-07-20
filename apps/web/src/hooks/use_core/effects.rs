// apps/web/src/hooks/use_core/effects.rs
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! 响应式效果入口。

mod handshake;
mod handshake_bootstrap;
mod message;
mod message_control;
mod message_control_runtime;
mod message_control_runtime_repo;
mod message_dispatch;
mod message_dispatch_control;
mod message_dispatch_gate;
mod message_dispatch_projection;
mod message_dispatch_protocol;
mod message_dispatch_route_control;
mod message_dispatch_route_projection;
mod message_dispatch_route_protocol;
mod message_dispatch_route_runtime;
mod message_dispatch_runtime;
mod message_dispatch_shadow;
mod message_dispatch_sync;
mod message_dispatch_write;
mod message_projection;
mod message_projection_recovery;
mod message_protocol;
mod message_refresh;
mod message_remove_scope;
mod message_repo_bootstrap;
mod message_repo_scope;
mod message_runtime;
mod message_runtime_remaining;
mod message_runtime_sync;
mod message_scope;
mod message_shadow;
mod message_sync;
mod message_sync_dispatch;

use crate::api::WsService;
use crate::runtime::repo_control_client::RepoControlClient;

use super::state::CoreSignals;
use super::types::HandshakeSignals;

/// 设置握手 Effect。
pub fn setup_handshake_effect(ws: &WsService, signals: HandshakeSignals) {
    handshake::setup(ws, signals);
}

/// 设置消息处理 Effect。
pub fn setup_message_effect(
    ws: &WsService,
    signals: &CoreSignals,
    external_changes_refresh: leptos::prelude::Callback<()>,
    repo_control: RepoControlClient,
) {
    message::setup(ws, signals, external_changes_refresh, repo_control);
}
