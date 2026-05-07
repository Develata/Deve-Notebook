//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::ws_protocol_acceptance_support::{
    WsHarness, connect_harness, recv_server_message, send_client_message, switch_to_notes_repo,
};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const SCOPE: u64 = 1;
type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_request_key_after_repo_switch_returns_repo_key() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = switched_ws(&harness).await?;

    send_client_message(
        &mut ws,
        ClientMessage::RequestKey {
            scope_nonce: Some(SCOPE),
        },
    )
    .await?;
    assert_key_provide(recv_server_message(&mut ws).await?, harness.repo_id);

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_request_key_rejects_missing_scope_nonce() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = switched_ws(&harness).await?;

    send_client_message(&mut ws, ClientMessage::RequestKey { scope_nonce: None }).await?;
    assert_protocol_error(
        recv_server_message(&mut ws).await?,
        Some(SCOPE),
        "request key scope nonce missing",
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_request_key_rejects_stale_scope_nonce() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = switched_ws(&harness).await?;

    send_client_message(
        &mut ws,
        ClientMessage::RequestKey {
            scope_nonce: Some(0),
        },
    )
    .await?;
    assert_protocol_error(
        recv_server_message(&mut ws).await?,
        Some(0),
        "request key scope nonce is stale",
    );

    harness.shutdown().await;
    Ok(())
}

async fn switched_ws(harness: &WsHarness) -> anyhow::Result<TestWs> {
    let mut ws = connect_harness(harness).await?;
    switch_to_notes_repo(&mut ws, harness.repo_id, SCOPE).await?;
    Ok(ws)
}

fn assert_key_provide(message: ServerMessage, repo_id: uuid::Uuid) {
    match message {
        ServerMessage::KeyProvide {
            repo_id: actual,
            scope_nonce,
            branch,
            repo_key,
        } => {
            assert_eq!(actual, repo_id);
            assert_eq!(scope_nonce, SCOPE);
            assert_eq!(branch, None);
            assert_eq!(repo_key.len(), 32);
        }
        other => panic!("expected KeyProvide, got {other:?}"),
    }
}

fn assert_protocol_error(message: ServerMessage, scope_nonce: Option<u64>, detail: &str) {
    match message {
        ServerMessage::ProtocolError {
            error,
            scope_nonce: actual_scope,
            ..
        } => {
            let expected_code = if detail.contains("stale") {
                ServerErrorCode::ScStaleScope
            } else {
                ServerErrorCode::ScRepoContextInvalid
            };
            assert_eq!(error.code, expected_code);
            assert_eq!(actual_scope, scope_nonce);
            assert!(error.detail.as_deref().is_some_and(|got| got.contains(detail)));
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
}
