use super::super::{LEGACY_JSON_TEXT_DISABLED_ERROR, SocketFlow, handle_incoming_message};
use super::build_state;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use crate::server::ws::filter::BroadcastFilter;
use axum::extract::ws::Message;
use deve_core::protocol::frame::{WS_PROTOCOL_VERSION, encode_client_binary_with_version};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_invalid_bincode_uses_current_scope_nonce_when_sync_scope_is_stale()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(17));
    session.set_sync_scope_nonce(23);

    let flow = handle_incoming_message(
        &state,
        &ch,
        &mut session,
        Message::Binary(vec![0, 1, 2]),
        &filter,
        "peer-1",
    )
    .await;

    assert!(matches!(flow, SocketFlow::Continue));
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(error.detail.as_deref(), Some("missing WS frame magic"));
            assert_eq!(scope_nonce, Some(17));
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
    Ok(())
}

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
async fn legacy_json_text_is_rejected_by_default_with_structured_error() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(41));
    let text = serde_json::to_string(&ClientMessage::Ping)?;

    let flow = handle_incoming_message(
        &state,
        &ch,
        &mut session,
        Message::Text(text),
        &filter,
        "peer-1",
    )
    .await;

    assert!(matches!(flow, SocketFlow::Continue));
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(
                error.detail.as_deref(),
                Some(LEGACY_JSON_TEXT_DISABLED_ERROR)
            );
            assert_eq!(scope_nonce, Some(41));
        }
        other => panic!("expected legacy text ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_protocol_version_reports_structured_error() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(31));
    let bytes = encode_client_binary_with_version(&ClientMessage::Ping, WS_PROTOCOL_VERSION - 1)?;

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
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(scope_nonce, Some(31));
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("unsupported WS protocol version"))
            );
        }
        other => panic!("expected protocol version error, got {:?}", other),
    }
    Ok(())
}
