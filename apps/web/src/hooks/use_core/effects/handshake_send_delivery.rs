//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::storage::identity::{StoredPeerIdentity, note_handshake, save_repo_vector};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::ClientMessage;
use std::collections::BTreeMap;

use super::super::handshake_state::reset_handshake_attempt;
use super::HandshakeAttemptCtx;

pub(super) fn build_handshake_message(
    peer_id: &str,
    vector: &VersionVector,
) -> Result<Vec<u8>, serde_json::Error> {
    let sorted_map: BTreeMap<_, _> = vector.iter().collect();
    let vec_bytes = serde_json::to_vec(&sorted_map)?;
    let mut msg = Vec::with_capacity(14 + peer_id.len() + vec_bytes.len());
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(peer_id.as_bytes());
    msg.extend_from_slice(&vec_bytes);
    Ok(msg)
}

pub(super) async fn deliver_signed_handshake(
    ctx: &HandshakeAttemptCtx,
    next_attempt: u64,
    identity: &StoredPeerIdentity,
    signature: Vec<u8>,
) {
    if ctx.handshake_attempt.get() != next_attempt {
        leptos::logging::log!("忽略过期握手结果: scope 已变更");
        return;
    }
    let peer_id = PeerId::new(&identity.peer_id);
    match uuid::Uuid::parse_str(&identity.repo_id) {
        Ok(repo_id) => {
            persist_repo_vector(&identity.repo_id, &ctx.vector).await;
            let _ = note_handshake(&identity.repo_id).await;
            let writer_peer_id = peer_id.clone();
            ctx.ws.send(ClientMessage::SyncHello {
                peer_id,
                pub_key: identity.public_key.clone(),
                signature,
                vector: ctx.vector.clone(),
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

async fn persist_repo_vector(repo_id: &str, vector: &VersionVector) {
    match serde_json::to_string(vector) {
        Ok(vector_json) => {
            let _ = save_repo_vector(repo_id, &vector_json).await;
        }
        Err(err) => {
            leptos::logging::warn!("保存握手向量失败: {}", err);
        }
    }
}
