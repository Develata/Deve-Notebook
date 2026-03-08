// apps/web/src/hooks/use_core/effects.rs
//! 响应式效果入口。

mod handshake;
mod message;

use crate::api::WsService;
use crate::storage::DegradedSyncMode;
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::VersionVector;
use leptos::prelude::*;

use super::state::CoreSignals;

/// 设置握手 Effect。
pub fn setup_handshake_effect(
    ws: &WsService,
    identity: ReadSignal<Option<StoredPeerIdentity>>,
    repo_vector: ReadSignal<VersionVector>,
    degraded: ReadSignal<Option<DegradedSyncMode>>,
    set_handshake_ready: WriteSignal<bool>,
) {
    handshake::setup(ws, identity, repo_vector, degraded, set_handshake_ready);
}

/// 设置消息处理 Effect。
pub fn setup_message_effect(ws: &WsService, signals: &CoreSignals) {
    message::setup(ws, signals);
}
