//! plan_ref:
//!   - 05_network#server-ws-runtime

use super::sync_hello_test_support::signed_hello_for_scope;
use super::ws_protocol_acceptance_support::{
    WsHarness, connect_harness, recv_server_message, send_client_message,
};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use deve_core::security::IdentityKeyPair;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const SCOPE: u64 = 1;
type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_register_writer_after_sync_hello_returns_write_ready() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let remote = IdentityKeyPair::generate();
    let mut ws = ready_ws(&harness, &remote).await?;
    send_writer(&mut ws, &remote, harness.repo_id, SCOPE).await?;
    match recv_server_message(&mut ws).await? {
        ServerMessage::WriteReady {
            peer_id,
            repo_id,
            scope_nonce,
            branch,
        } => assert_eq!(
            (peer_id, repo_id, scope_nonce, branch),
            (remote.peer_id(), harness.repo_id, SCOPE, None)
        ),
        other => panic!("expected WriteReady, got {other:?}"),
    }
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_register_writer_rejects_wrong_repo() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let remote = IdentityKeyPair::generate();
    let mut ws = ready_ws(&harness, &remote).await?;
    send_writer(&mut ws, &remote, uuid::Uuid::new_v4(), SCOPE).await?;
    assert_protocol_error(
        recv_server_message(&mut ws).await?,
        SCOPE,
        "writer scope does not match active repo",
    );
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_register_writer_rejects_stale_scope_nonce() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let remote = IdentityKeyPair::generate();
    let mut ws = ready_ws(&harness, &remote).await?;
    send_writer(&mut ws, &remote, harness.repo_id, 0).await?;
    assert_protocol_error(
        recv_server_message(&mut ws).await?,
        0,
        "writer scope nonce is stale",
    );
    harness.shutdown().await;
    Ok(())
}

async fn ready_ws(harness: &WsHarness, remote: &IdentityKeyPair) -> anyhow::Result<TestWs> {
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
    send_client_message(&mut ws, client_sync_hello(remote, harness.repo_id)).await?;
    let _ = recv_server_message(&mut ws).await?;
    let _ = recv_server_message(&mut ws).await?;
    Ok(ws)
}

fn client_sync_hello(remote: &IdentityKeyPair, repo_id: uuid::Uuid) -> ClientMessage {
    let hello = signed_hello_for_scope(remote, repo_id, SCOPE);
    ClientMessage::SyncHello {
        peer_id: hello.peer_id,
        peer_pubkey: hello.peer_pubkey,
        session_proof: hello.session_proof,
        vector: hello.remote_vector,
        repo_id: hello.repo_id,
        scope_nonce: hello.scope_nonce,
    }
}

async fn send_writer(
    ws: &mut TestWs,
    remote: &IdentityKeyPair,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> anyhow::Result<()> {
    send_client_message(
        ws,
        ClientMessage::RegisterWriter {
            peer_id: remote.peer_id(),
            repo_id,
            scope_nonce,
        },
    )
    .await
}

fn assert_protocol_error(message: ServerMessage, expected_scope: u64, detail: &str) {
    let ServerMessage::ProtocolError {
        error, scope_nonce, ..
    } = message
    else {
        panic!("expected ProtocolError, got {message:?}");
    };
    let expected_code = if detail.contains("stale") {
        ServerErrorCode::ScStaleScope
    } else {
        ServerErrorCode::ScRepoContextInvalid
    };
    assert_eq!(error.code, expected_code);
    assert_eq!(scope_nonce, Some(expected_scope));
    assert!(error.detail.as_deref().is_some_and(|got| got == detail));
}
