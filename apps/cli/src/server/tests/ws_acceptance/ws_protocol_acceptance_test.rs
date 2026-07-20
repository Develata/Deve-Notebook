//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! End-to-end WebSocket protocol frame acceptance coverage.

use super::ws_protocol_acceptance_support::{
    connect_harness, recv_server_message, send_client_message, WsHarness,
};
use crate::server::ws::WS_JSON_TEXT_ENV_LOCK;
use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse, AuthStatusResponse};
use deve_core::protocol::frame::{
    encode_client_binary_with_version, ClientFrame, WS_PROTOCOL_VERSION,
};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use futures::SinkExt;
use reqwest::header::{CONNECTION, SET_COOKIE, UPGRADE};
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
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
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
async fn anonymous_localhost_status_sets_dev_session_cookie() -> anyhow::Result<()> {
    // AUTH-014: anonymous localhost keeps quiet auth probe semantics while
    // establishing a browser-session cookie for HTTP/WS grant binding.
    let harness = WsHarness::spawn_with_anonymous_localhost(true).await?;
    let client = reqwest::Client::builder().no_proxy().build()?;
    let url = harness
        .ws_url
        .replacen("ws://", "http://", 1)
        .replace("/ws", "/api/auth/status");
    let response = client.get(url).send().await?;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let set_cookie = response
        .headers()
        .get(SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .expect("dev session set-cookie");
    assert!(set_cookie.starts_with("deve_dev_session="));
    assert!(set_cookie.contains("HttpOnly"));
    assert_eq!(
        response.json::<AuthStatusResponse>().await?,
        AuthStatusResponse::authenticated()
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
            assert_eq!(error.code, ServerErrorCode::SyncVersionMismatch);
            assert!(error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("unsupported WS protocol version")));
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
    let legacy_bytes = vec![0_u8, 1, 2, 3];

    ws.send(Message::Binary(legacy_bytes)).await?;

    match recv_server_message(&mut ws).await? {
        ServerMessage::ProtocolError { error, .. } => {
            assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
            assert_eq!(error.detail.as_deref(), Some("missing WS frame magic"));
        }
        other => panic!("expected legacy binary ProtocolError, got {other:?}"),
    }
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_rejects_versioned_json_text_by_default() -> anyhow::Result<()> {
    let _lock = WS_JSON_TEXT_ENV_LOCK.lock().await;
    let _env = EnvGuard::set_many(&[
        ("DEVE_ENV", Some("production")),
        ("DEVE_ALLOW_WS_JSON_TEXT", None),
    ]);
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    let text = serde_json::to_string(&ClientFrame::current(ClientMessage::Ping))?;

    ws.send(Message::Text(text)).await?;

    assert_json_text_disabled(recv_server_message(&mut ws).await?);
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_accepts_versioned_json_text_when_debug_enabled() -> anyhow::Result<()> {
    let _lock = WS_JSON_TEXT_ENV_LOCK.lock().await;
    let _env = EnvGuard::set_many(&[
        ("DEVE_ENV", Some("production")),
        ("DEVE_ALLOW_WS_JSON_TEXT", Some("1")),
    ]);
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    let frame = ClientFrame::current(ClientMessage::Ping);
    assert_eq!(frame.protocol_version, 4);
    let text = serde_json::to_string(&frame)?;

    ws.send(Message::Text(text)).await?;

    assert!(matches!(
        recv_server_message(&mut ws).await?,
        ServerMessage::Pong
    ));
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_rejects_unversioned_json_text_when_debug_enabled() -> anyhow::Result<()> {
    let _lock = WS_JSON_TEXT_ENV_LOCK.lock().await;
    let _env = EnvGuard::set_many(&[
        ("DEVE_ENV", Some("production")),
        ("DEVE_ALLOW_WS_JSON_TEXT", Some("1")),
    ]);
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    let text = serde_json::to_string(&ClientMessage::Ping)?;

    ws.send(Message::Text(text)).await?;

    match recv_server_message(&mut ws).await? {
        ServerMessage::ProtocolError { error, .. } => {
            assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
            assert_eq!(error.detail.as_deref(), Some("Invalid JSON client message"));
        }
        other => panic!("expected invalid JSON ProtocolError, got {other:?}"),
    }
    harness.shutdown().await;
    Ok(())
}

fn assert_json_text_disabled(message: ServerMessage) {
    match message {
        ServerMessage::ProtocolError { error, .. } => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(
                error.detail.as_deref(),
                Some("JSON WS text frames are disabled outside development debug mode")
            );
        }
        other => panic!("expected JSON text ProtocolError, got {other:?}"),
    }
}

struct EnvGuard {
    old: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set_many(vars: &[(&'static str, Option<&str>)]) -> Self {
        let old = vars
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in vars {
            // SAFETY: these tests hold WS_JSON_TEXT_ENV_LOCK while mutating these env keys.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
        Self { old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.old.drain(..) {
            // SAFETY: EnvGuard restores only keys it changed while the async env lock is held.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
