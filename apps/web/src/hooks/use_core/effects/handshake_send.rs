use crate::api::WsService;
use crate::storage::DegradedSyncMode;
use crate::storage::identity::{StoredPeerIdentity, sign_sync_hello};
use deve_core::models::{PeerId, VersionVector};
use leptos::prelude::Set;
use leptos::task::spawn_local;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::super::super::types::HandshakeSignals;
use super::handshake_reset::restore_scope_if_needed;
use super::handshake_state::{reset_handshake_attempt, set_handshake_scope_nonce_if_changed};
#[path = "handshake_send_delivery.rs"]
mod delivery;

pub(super) struct HandshakeAttemptCtx {
    pub ws: WsService,
    pub signals: HandshakeSignals,
    pub maybe_mode: Option<DegradedSyncMode>,
    pub maybe_identity: Option<StoredPeerIdentity>,
    pub vector: VersionVector,
    pub repo_name: Option<String>,
    pub active_repo_id: Option<String>,
    pub branch: Option<PeerId>,
    pub current_scope_nonce: u64,
    pub should_restore: bool,
    pub handshake_attempt: Rc<Cell<u64>>,
    pub failure_last_mode: Rc<RefCell<Option<String>>>,
}

pub(super) fn spawn_handshake_attempt(ctx: HandshakeAttemptCtx) {
    let next_attempt = ctx.handshake_attempt.get().saturating_add(1);
    ctx.handshake_attempt.set(next_attempt);
    spawn_local(async move {
        if let Some(mode) = ctx.maybe_mode {
            leptos::logging::warn!("{}", mode.banner_text());
            restore_scope_if_needed(
                &ctx.ws,
                ctx.signals,
                ctx.should_restore,
                ctx.repo_name.clone(),
                ctx.active_repo_id.clone(),
                ctx.branch.clone(),
            );
            ctx.ws.clear_writer_ready();
            ctx.signals.set_handshake_ready.set(true);
            set_handshake_scope_nonce_if_changed(ctx.signals, None);
            return;
        }
        let Some(ref identity) = ctx.maybe_identity else {
            return;
        };

        restore_scope_if_needed(
            &ctx.ws,
            ctx.signals,
            ctx.should_restore,
            ctx.repo_name.clone(),
            ctx.active_repo_id.clone(),
            ctx.branch.clone(),
        );
        let msg = match delivery::build_handshake_message(&identity.peer_id, &ctx.vector) {
            Ok(msg) => msg,
            Err(err) => {
                leptos::logging::error!("序列化握手向量失败: {}", err);
                reset_handshake_attempt(&ctx.failure_last_mode, &ctx.ws, ctx.signals);
                return;
            }
        };

        match sign_sync_hello(&identity, &msg).await {
            Ok(signature) => {
                delivery::deliver_signed_handshake(&ctx, next_attempt, &identity, signature).await;
            }
            Err(err) => {
                leptos::logging::error!("WebCrypto 握手签名失败: {}", err);
                reset_handshake_attempt(&ctx.failure_last_mode, &ctx.ws, ctx.signals);
            }
        }
    });
}
