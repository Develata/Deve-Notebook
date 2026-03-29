// apps/web/src/hooks/use_core/effects.rs
//! 响应式效果入口。

mod handshake;
mod handshake_bootstrap;
mod message;
mod message_control;
mod message_control_runtime;
mod message_dispatch;
mod message_dispatch_control;
mod message_dispatch_gate;
mod message_dispatch_projection;
mod message_dispatch_protocol;
mod message_dispatch_route_control;
mod message_dispatch_route_projection;
mod message_dispatch_route_protocol;
mod message_dispatch_runtime;
mod message_dispatch_shadow;
mod message_dispatch_sync;
mod message_dispatch_write;
mod message_projection;
mod message_protocol;
mod message_refresh;
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

use super::state::CoreSignals;
use super::types::HandshakeSignals;

/// 设置握手 Effect。
pub fn setup_handshake_effect(ws: &WsService, signals: HandshakeSignals) {
    handshake::setup(ws, signals);
}

/// 设置消息处理 Effect。
pub fn setup_message_effect(ws: &WsService, signals: &CoreSignals) {
    message::setup(ws, signals);
}
