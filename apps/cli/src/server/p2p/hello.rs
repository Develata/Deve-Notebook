//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 07_network#full-peer-ws-admission

use crate::server::AppState;
use anyhow::{Context, Result};
use deve_core::config::P2pPeerConfig;
use deve_core::models::{RepoId, VersionVector};
use deve_core::protocol::{ClientMessage, ScopeNonce, SessionProof};
use deve_core::security::IdentityKeyPair;
use deve_core::sync::handshake_proof::sign_sync_hello;
use std::sync::Arc;

pub(super) fn parse_repo_id(peer: &P2pPeerConfig) -> Result<RepoId> {
    uuid::Uuid::parse_str(&peer.repo_id)
        .with_context(|| format!("Invalid P2P repo_id for peer {}", peer.label))
}

pub(super) fn build_sync_hello(state: &Arc<AppState>, repo_id: RepoId) -> Result<ClientMessage> {
    let vector = state
        .sync_engine
        .with_strict_engine(repo_id, |engine| engine.version_vector().clone())
        .with_context(|| {
            format!("Failed to refresh local vector before P2P hello for {repo_id}")
        })?;
    signed_sync_hello(state.identity_key.as_ref(), repo_id, vector)
        .with_context(|| format!("Failed to sign P2P SyncHello for {repo_id}"))
}

pub(super) fn signed_sync_hello(
    identity: &IdentityKeyPair,
    repo_id: RepoId,
    vector: VersionVector,
) -> Result<ClientMessage> {
    let peer_id = identity.peer_id();
    let signature = sign_sync_hello(identity, &vector)?;

    Ok(ClientMessage::SyncHello {
        peer_id,
        peer_pubkey: identity.public_key_bytes().to_vec(),
        session_proof: SessionProof::new(signature),
        vector,
        repo_id,
        scope_nonce: ScopeNonce::new(0),
    })
}
