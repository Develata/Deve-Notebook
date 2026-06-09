//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Sync hello local response signing.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::models::{RepoId, VersionVector};
use deve_core::protocol::ServerMessage;
use deve_core::sync::handshake_proof::sign_sync_hello;
use std::sync::Arc;

pub(super) fn send(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_id: RepoId,
    scope_nonce: u64,
    local_vector: VersionVector,
) -> Result<(), serde_json::Error> {
    let local_peer_id = state.identity_key.peer_id();
    let my_sig = sign_sync_hello(state.identity_key.as_ref(), &local_vector)?;
    ch.unicast(ServerMessage::SyncHello {
        peer_id: local_peer_id,
        repo_id,
        scope_nonce: scope_nonce.into(),
        pub_key: state.identity_key.public_key_bytes().to_vec(),
        signature: my_sig,
        vector: local_vector,
    });
    Ok(())
}
