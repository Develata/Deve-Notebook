use super::super::{JSON_TEXT_DISABLED_ERROR, SocketFlow, handle_incoming_message};
use super::build_state;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use crate::server::ws::WS_JSON_TEXT_ENV_LOCK;
use crate::server::ws::filter::BroadcastFilter;
use axum::extract::ws::Message;
use deve_core::protocol::frame::{
    ClientFrame, MIN_SUPPORTED_WS_PROTOCOL_VERSION, WS_PROTOCOL_VERSION,
    encode_client_binary_with_version,
};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_invalid_binary_uses_current_scope_nonce_when_sync_scope_is_stale()
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
            assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
            assert_eq!(error.detail.as_deref(), Some("missing WS frame magic"));
            assert_eq!(scope_nonce, Some(17));
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn versioned_json_text_is_debug_gated_with_structured_error() -> anyhow::Result<()> {
    let _lock = WS_JSON_TEXT_ENV_LOCK.lock().await;
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(41));
    let text = serde_json::to_string(&ClientFrame {
        protocol_version: WS_PROTOCOL_VERSION,
        message: ClientMessage::Ping,
    })?;

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
            assert_eq!(error.detail.as_deref(), Some(JSON_TEXT_DISABLED_ERROR));
            assert_eq!(scope_nonce, Some(41));
        }
        other => panic!("expected debug text ProtocolError, got {:?}", other),
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
    let bytes = encode_client_binary_with_version(
        &ClientMessage::Ping,
        MIN_SUPPORTED_WS_PROTOCOL_VERSION - 1,
    )?;

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
            assert_eq!(error.code, ServerErrorCode::SyncVersionMismatch);
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_versioned_json_reports_version_mismatch() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(37));
    let text = serde_json::to_string(&ClientFrame {
        protocol_version: MIN_SUPPORTED_WS_PROTOCOL_VERSION - 1,
        message: ClientMessage::Ping,
    })?;

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
            assert_eq!(error.code, ServerErrorCode::SyncVersionMismatch);
            assert_eq!(scope_nonce, Some(37));
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("unsupported WS protocol version"))
            );
        }
        other => panic!("expected JSON protocol version error, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_versioned_binary_reports_invalid_payload() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(43));
    let mut bytes = encode_client_binary_with_version(&ClientMessage::Ping, WS_PROTOCOL_VERSION)?;
    bytes.pop();

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
            assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
            assert_eq!(
                error.detail.as_deref(),
                Some("Invalid binary client message")
            );
            assert_eq!(scope_nonce, Some(43));
        }
        other => panic!("expected malformed binary ProtocolError, got {:?}", other),
    }
    Ok(())
}
