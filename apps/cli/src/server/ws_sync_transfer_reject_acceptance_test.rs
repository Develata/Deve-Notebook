//! plan_ref:
//!   - 05_network#server-ws-runtime

use super::sync_hello_test_support::signed_hello_for_scope;
use super::ws_protocol_acceptance_support::{
    WsHarness, connect_harness, recv_server_message, send_client_message,
};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use deve_core::security::IdentityKeyPair;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const SCOPE: u64 = 1;
type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_sync_request_requires_sync_hello_scope() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = switched_ws(&harness).await?;
    expect_reject(
        &mut ws,
        sync_request(harness.repo_id),
        ServerErrorCode::ScRepoContextInvalid,
        "browser sync scope not bound",
    )
    .await?;
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_sync_request_rejects_wrong_repo_after_sync_hello() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = ready_ws(&harness).await?;
    expect_reject(
        &mut ws,
        sync_request(uuid::Uuid::new_v4()),
        ServerErrorCode::SyncRepoRouteMismatch,
        "",
    )
    .await?;
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_sync_push_rejects_wrong_repo_after_sync_hello() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = ready_ws(&harness).await?;
    expect_reject(
        &mut ws,
        sync_push(PeerId::new("origin"), uuid::Uuid::new_v4()),
        ServerErrorCode::SyncRepoRouteMismatch,
        "",
    )
    .await?;
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_sync_push_rejects_unrequested_source() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = ready_ws(&harness).await?;
    expect_reject(
        &mut ws,
        sync_push(PeerId::new("unrequested-source"), harness.repo_id),
        ServerErrorCode::SyncPeerUnauthenticated,
        "was not requested from transport",
    )
    .await?;
    harness.shutdown().await;
    Ok(())
}

async fn ready_ws(harness: &WsHarness) -> anyhow::Result<TestWs> {
    let remote = IdentityKeyPair::generate();
    let mut ws = switched_ws(harness).await?;
    send_client_message(&mut ws, client_sync_hello(&remote, harness.repo_id)).await?;
    let _ = recv_server_message(&mut ws).await?;
    let _ = recv_server_message(&mut ws).await?;
    Ok(ws)
}

async fn switched_ws(harness: &WsHarness) -> anyhow::Result<TestWs> {
    let mut ws = connect_harness(harness).await?;
    send_client_message(
        &mut ws,
        ClientMessage::SwitchRepoExact {
            name: "notes".into(),
            repo_id: harness.repo_id,
            switch_nonce: Some(SCOPE),
        },
    )
    .await?;
    let _ = recv_server_message(&mut ws).await?;
    let _ = recv_server_message(&mut ws).await?;
    let _ = recv_server_message(&mut ws).await?;
    Ok(ws)
}

fn client_sync_hello(remote: &IdentityKeyPair, repo_id: uuid::Uuid) -> ClientMessage {
    let hello = signed_hello_for_scope(remote, repo_id, SCOPE);
    ClientMessage::SyncHello {
        peer_id: hello.peer_id,
        pub_key: hello.pub_key,
        signature: hello.signature,
        vector: hello.remote_vector,
        repo_id: hello.repo_id,
        scope_nonce: hello.scope_nonce,
    }
}

async fn expect_reject(
    ws: &mut TestWs,
    msg: ClientMessage,
    code: ServerErrorCode,
    detail: &str,
) -> anyhow::Result<()> {
    send_client_message(ws, msg).await?;
    assert_protocol_error(recv_server_message(ws).await?, code, detail);
    Ok(())
}

fn sync_request(repo_id: uuid::Uuid) -> ClientMessage {
    ClientMessage::SyncRequest {
        repo_id,
        known_vector: VersionVector::new(),
        requests: vec![],
    }
}

fn sync_push(peer_id: PeerId, repo_id: uuid::Uuid) -> ClientMessage {
    ClientMessage::SyncPush {
        peer_id,
        repo_id,
        ops: vec![],
    }
}

fn assert_protocol_error(message: ServerMessage, code: ServerErrorCode, detail: &str) {
    let ServerMessage::ProtocolError {
        error, scope_nonce, ..
    } = message
    else {
        panic!("expected ProtocolError, got {message:?}");
    };
    assert_eq!(error.code, code);
    assert_eq!(scope_nonce, Some(SCOPE));
    if detail.is_empty() {
        assert_eq!(error.detail, None);
    } else {
        assert!(error.detail.as_deref().is_some_and(|got| got.contains(detail)));
    }
}
