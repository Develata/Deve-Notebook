//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 07_network#full-peer-ws-admission

use crate::server::AppState;
use crate::server::p2p::exchange::drive_sync_exchange;
use crate::server::p2p::hello::{build_sync_hello, parse_repo_id};
use crate::server::p2p::stats::ExchangeStats;
use anyhow::{Context, Result, anyhow};
use deve_core::config::P2pPeerConfig;
use deve_core::models::PeerId;
use deve_core::protocol::frame::encode_client_binary;
use futures::SinkExt;
use std::sync::Arc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

pub(in crate::server) async fn connect_peer_once(
    peer: &P2pPeerConfig,
    state: Arc<AppState>,
) -> Result<ExchangeStats> {
    let token = std::env::var(&peer.auth_token_env)
        .with_context(|| format!("P2P token env is missing for peer {}", peer.label))?;
    if token.is_empty() {
        return Err(anyhow!("P2P token env is empty for peer {}", peer.label));
    }

    let mut request = peer
        .ws_url
        .as_str()
        .into_client_request()
        .with_context(|| format!("Invalid P2P ws_url for peer {}", peer.label))?;
    let auth_header = HeaderValue::from_str(&format!("Bearer {token}"))
        .with_context(|| format!("Invalid P2P bearer header for peer {}", peer.label))?;
    request.headers_mut().insert("authorization", auth_header);

    let repo_id = parse_repo_id(peer)?;
    let hello = build_sync_hello(&state, repo_id)?;
    let encoded = encode_client_binary(&hello).context("Failed to encode P2P SyncHello")?;

    let (mut socket, _) = connect_async(request)
        .await
        .with_context(|| format!("Failed to connect P2P peer {}", peer.label))?;
    socket
        .send(Message::Binary(encoded))
        .await
        .with_context(|| format!("Failed to send SyncHello to P2P peer {}", peer.label))?;

    let stats = drive_sync_exchange(peer, repo_id, state, &mut socket).await?;
    tracing::info!(
        peer_label = %peer.label,
        peer_id = %peer.peer_id,
        authenticated_peer_id = stats
            .authenticated_peer_id
            .as_ref()
            .map(PeerId::as_str)
            .unwrap_or("unknown"),
        repo_id = %repo_id,
        sent_pushes = stats.sent_pushes,
        sent_snapshots = stats.sent_snapshots,
        applied_pushes = stats.applied_pushes,
        applied_snapshots = stats.applied_snapshots,
        "P2P mesh connector handshake completed"
    );
    Ok(stats)
}
