use crate::api::WsService;
use crate::storage::DegradedSyncMode;
use crate::storage::identity::{
    StoredPeerIdentity, note_handshake, save_repo_vector, sign_sync_hello,
};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::ClientMessage;
use leptos::prelude::Set;
use leptos::task::spawn_local;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::super::super::types::HandshakeSignals;
use super::handshake_reset::restore_scope_if_needed;
use super::handshake_state::{reset_handshake_attempt, set_handshake_scope_nonce_if_changed};

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
        let Some(identity) = ctx.maybe_identity else {
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
        let sorted_map: BTreeMap<_, _> = ctx.vector.iter().collect();
        let vec_bytes = match serde_json::to_vec(&sorted_map) {
            Ok(bytes) => bytes,
            Err(err) => {
                leptos::logging::error!("序列化握手向量失败: {}", err);
                reset_handshake_attempt(&ctx.failure_last_mode, &ctx.ws, ctx.signals);
                return;
            }
        };
        let mut msg = Vec::new();
        msg.extend_from_slice(b"deve-handshake");
        msg.extend_from_slice(identity.peer_id.as_bytes());
        msg.extend_from_slice(&vec_bytes);

        match sign_sync_hello(&identity, &msg).await {
            Ok(signature) => {
                if ctx.handshake_attempt.get() != next_attempt {
                    leptos::logging::log!("忽略过期握手结果: scope 已变更");
                    return;
                }
                let peer_id = PeerId::new(&identity.peer_id);
                match uuid::Uuid::parse_str(&identity.repo_id) {
                    Ok(repo_id) => {
                        match serde_json::to_string(&ctx.vector) {
                            Ok(vector_json) => {
                                let _ = save_repo_vector(&identity.repo_id, &vector_json).await;
                            }
                            Err(err) => {
                                leptos::logging::warn!("保存握手向量失败: {}", err);
                            }
                        }
                        let _ = note_handshake(&identity.repo_id).await;
                        let writer_peer_id = peer_id.clone();
                        ctx.ws.send(ClientMessage::SyncHello {
                            peer_id,
                            pub_key: identity.public_key.clone(),
                            signature,
                            vector: ctx.vector,
                            repo_id,
                            scope_nonce: ctx.current_scope_nonce,
                        });
                        ctx.ws.send(ClientMessage::RegisterWriter {
                            peer_id: writer_peer_id,
                            repo_id,
                            scope_nonce: ctx.current_scope_nonce,
                        });
                    }
                    Err(err) => {
                        leptos::logging::error!(
                            "跳过 SyncHello: 非法 repo_id {} ({})",
                            identity.repo_id,
                            err
                        );
                        reset_handshake_attempt(&ctx.failure_last_mode, &ctx.ws, ctx.signals);
                    }
                }
            }
            Err(err) => {
                leptos::logging::error!("WebCrypto 握手签名失败: {}", err);
                reset_handshake_attempt(&ctx.failure_last_mode, &ctx.ws, ctx.signals);
            }
        }
    });
}
