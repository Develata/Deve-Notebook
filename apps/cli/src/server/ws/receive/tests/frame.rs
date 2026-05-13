use super::super::{SocketFlow, handle_incoming_message};
use super::build_state;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use crate::server::ws::filter::BroadcastFilter;
use axum::extract::ws::Message;
use deve_core::protocol::frame::{
    MIN_SUPPORTED_WS_PROTOCOL_VERSION, WS_PROTOCOL_VERSION, encode_client_binary_with_version,
};
use deve_core::protocol::{ClientMessage, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn versioned_binary_ping_routes_to_pong() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    let bytes = encode_client_binary_with_version(&ClientMessage::Ping, WS_PROTOCOL_VERSION)?;

    let flow = handle_incoming_message(
        &state,
        &ch,
        &mut session,
        Message::Binary(bytes),
        &filter,
        "peer-1",
    )
    .await;

    assert!(matches!(flow, SocketFlow::Continue));
    assert!(matches!(uni_rx.recv().await, Some(ServerMessage::Pong)));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn minimum_supported_binary_ping_routes_to_pong() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    let bytes =
        encode_client_binary_with_version(&ClientMessage::Ping, MIN_SUPPORTED_WS_PROTOCOL_VERSION)?;

    let flow = handle_incoming_message(
        &state,
        &ch,
        &mut session,
        Message::Binary(bytes),
        &filter,
        "peer-1",
    )
    .await;

    assert!(matches!(flow, SocketFlow::Continue));
    assert!(matches!(uni_rx.recv().await, Some(ServerMessage::Pong)));
    Ok(())
}
