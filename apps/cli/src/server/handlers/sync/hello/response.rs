//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Sync hello local response signing.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) fn send(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_id: RepoId,
    scope_nonce: u64,
    local_peer_id: PeerId,
    local_vector: VersionVector,
) -> Result<(), serde_json::Error> {
    let vec_bytes = serde_json::to_vec(&local_vector)?;
    let mut msg = Vec::new();
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(local_peer_id.as_str().as_bytes());
    msg.extend_from_slice(&vec_bytes);

    let my_sig = state.identity_key.sign(&msg);
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
