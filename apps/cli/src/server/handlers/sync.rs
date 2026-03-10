//! P2P 同步消息处理器入口。

#![allow(dead_code)]

mod errors;
mod guard;
mod hello;
mod snapshot;
mod transfer;
mod writer;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::security::EncryptedOp;
use std::sync::Arc;

pub use hello::SyncHelloInput;

pub async fn handle_sync_hello(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    hello: SyncHelloInput,
) {
    hello::handle(state, ch, session, hello).await;
}

pub fn handle_register_writer(
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    peer_id: PeerId,
) {
    writer::handle(ch, session, repo_id, peer_id);
}

pub async fn handle_sync_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    repo_id: RepoId,
    requests: Vec<(PeerId, (u64, u64))>,
) {
    transfer::handle_request(state, ch, session, repo_id, requests).await;
}

pub async fn handle_sync_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) {
    transfer::handle_push(state, ch, session, repo_id, ops).await;
}

pub async fn handle_sync_snapshot_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    peer_id: PeerId,
    repo_id: RepoId,
) {
    snapshot::handle_request(state, ch, session, peer_id, repo_id).await;
}

pub async fn handle_sync_push_snapshot(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) {
    snapshot::handle_push(state, ch, session, peer_id, repo_id, ops).await;
}

pub async fn handle_delete_peer(state: &Arc<AppState>, ch: &DualChannel, peer_id_str: String) {
    let peer_id = PeerId::new(peer_id_str.clone());
    tracing::info!("Handling DeletePeer request for: {}", peer_id);

    match state.repo.delete_peer_branch(&peer_id) {
        Ok(_) => {
            tracing::info!("Successfully deleted peer branch: {}", peer_id);
            ch.broadcast(deve_core::protocol::ServerMessage::PeerDeleted {
                peer_id: peer_id_str,
            });
            crate::server::handlers::listing::handle_list_shadows(state, ch).await;
        }
        Err(e) => {
            tracing::error!("Failed to delete peer branch {}: {:?}", peer_id, e);
            errors::storage_persist_failed(ch, format!("Failed to delete peer: {}", e));
        }
    }
}
