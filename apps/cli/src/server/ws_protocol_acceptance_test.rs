//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! End-to-end WebSocket protocol frame acceptance coverage.

use super::ws_protocol_acceptance_support::{
    WsHarness, connect_harness, recv_server_message, send_client_message,
};
use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse, AuthStatusResponse};
use deve_core::protocol::frame::{
    ClientFrame, WS_PROTOCOL_VERSION, encode_client_binary_with_version,
};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use futures::SinkExt;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_roundtrips_versioned_binary_ping() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    send_client_message(&mut ws, ClientMessage::Ping).await?;

    assert!(matches!(
        recv_server_message(&mut ws).await?,
        ServerMessage::Pong
    ));
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_unauthorized_response_is_structured_json() -> anyhow::Result<()> {
    let harness = WsHarness::spawn_with_anonymous_localhost(false).await?;
    let client = reqwest::Client::builder().no_proxy().build()?;
    let response = client
        .get(harness.ws_url.replacen("ws://", "http://", 1))
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    let payload = response.json::<AuthErrorResponse>().await?;
    assert_eq!(payload.code, AuthErrorCode::TokenMissing);
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_status_endpoint_is_public_and_quiet_when_missing_token() -> anyhow::Result<()> {
    // AUTH-012: public session probe must not produce unauthenticated 401 noise.
    let harness = WsHarness::spawn_with_anonymous_localhost(false).await?;
    let client = reqwest::Client::builder().no_proxy().build()?;
    let url = harness
        .ws_url
        .replacen("ws://", "http://", 1)
        .replace("/ws", "/api/auth/status");
    let response = client.get(url).send().await?;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response.json::<AuthStatusResponse>().await?,
        AuthStatusResponse::unauthenticated()
    );
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_rejects_unsupported_protocol_version() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    let unsupported_version = WS_PROTOCOL_VERSION + 1;
    let bytes = encode_client_binary_with_version(&ClientMessage::Ping, unsupported_version)?;

    ws.send(Message::Binary(bytes)).await?;

    match recv_server_message(&mut ws).await? {
        ServerMessage::ProtocolError { error, .. } => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("unsupported WS protocol version"))
            );
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_rejects_legacy_binary_without_magic() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    let legacy_bytes = bincode::serialize(&ClientMessage::Ping)?;

    ws.send(Message::Binary(legacy_bytes)).await?;

    match recv_server_message(&mut ws).await? {
        ServerMessage::ProtocolError { error, .. } => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(
                error.detail.as_deref(),
                Some("Invalid bincode client message")
            );
        }
        other => panic!("expected legacy binary ProtocolError, got {other:?}"),
    }
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_accepts_versioned_json_text_debug_frame() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    let text = serde_json::to_string(&ClientFrame::current(ClientMessage::Ping))?;

    ws.send(Message::Text(text)).await?;

    assert!(matches!(
        recv_server_message(&mut ws).await?,
        ServerMessage::Pong
    ));
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_accepts_legacy_json_text_debug_frame() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    let text = serde_json::to_string(&ClientMessage::Ping)?;

    ws.send(Message::Text(text)).await?;

    assert!(matches!(
        recv_server_message(&mut ws).await?,
        ServerMessage::Pong
    ));
    harness.shutdown().await;
    Ok(())
}
