// apps/web/src/hooks/use_core/effects.rs
//! 响应式效果入口。

mod handshake;
mod handshake_bootstrap;
mod message;
mod message_dispatch;
mod message_protocol;
mod message_sync;

use crate::api::WsService;
use crate::storage::DegradedSyncMode;
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::{PeerId, VersionVector};
use leptos::prelude::*;

use super::state::CoreSignals;

/// 设置握手 Effect。
pub fn setup_handshake_effect(
    ws: &WsService,
    identity: ReadSignal<Option<StoredPeerIdentity>>,
    repo_vector: ReadSignal<VersionVector>,
    degraded: ReadSignal<Option<DegradedSyncMode>>,
    current_repo: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<PeerId>>,
    set_handshake_ready: WriteSignal<bool>,
) {
    handshake::setup(
        ws,
        identity,
        repo_vector,
        degraded,
        current_repo,
        active_branch,
        set_handshake_ready,
    );
}

/// 设置消息处理 Effect。
pub fn setup_message_effect(ws: &WsService, signals: &CoreSignals) {
    message::setup(ws, signals);
}
