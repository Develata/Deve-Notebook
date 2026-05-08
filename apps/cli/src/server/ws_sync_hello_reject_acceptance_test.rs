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

const SWITCH_NONCE: u64 = 1;
const STALE_NONCE: u64 = 0;
type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_rejects_sync_hello_before_repo_switch() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    let remote = IdentityKeyPair::generate();

    expect_sync_hello_reject(
        &mut ws,
        &remote,
        harness.repo_id,
        SWITCH_NONCE,
        ServerErrorCode::ScRepoContextInvalid,
        "scope mismatch",
    )
    .await?;

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_rejects_sync_hello_for_wrong_repo() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    switch_repo(&mut ws, harness.repo_id).await?;
    let remote = IdentityKeyPair::generate();

    expect_sync_hello_reject(
        &mut ws,
        &remote,
        uuid::Uuid::new_v4(),
        SWITCH_NONCE,
        ServerErrorCode::ScRepoContextInvalid,
        "scope mismatch",
    )
    .await?;

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_rejects_sync_hello_with_stale_scope_nonce() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = connect_harness(&harness).await?;
    switch_repo(&mut ws, harness.repo_id).await?;
    let remote = IdentityKeyPair::generate();

    expect_sync_hello_reject(
        &mut ws,
        &remote,
        harness.repo_id,
        STALE_NONCE,
        ServerErrorCode::ScStaleScope,
        "stale scope nonce",
    )
    .await?;

    harness.shutdown().await;
    Ok(())
}

async fn expect_sync_hello_reject(
    ws: &mut TestWs,
    remote: &IdentityKeyPair,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
    expected_code: ServerErrorCode,
    detail: &str,
) -> anyhow::Result<()> {
    send_client_message(ws, client_sync_hello(remote, repo_id, scope_nonce)).await?;
    assert_protocol_error(
        recv_server_message(ws).await?,
        scope_nonce,
        expected_code,
        detail,
    );
    Ok(())
}

fn client_sync_hello(
    remote: &IdentityKeyPair,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> ClientMessage {
    let hello = signed_hello_for_scope(remote, repo_id, scope_nonce);
    ClientMessage::SyncHello {
        peer_id: hello.peer_id,
        peer_pubkey: hello.peer_pubkey,
        session_proof: hello.session_proof,
        vector: hello.remote_vector,
        repo_id: hello.repo_id,
        scope_nonce: hello.scope_nonce,
    }
}

async fn switch_repo(ws: &mut TestWs, repo_id: uuid::Uuid) -> anyhow::Result<()> {
    send_client_message(
        ws,
        ClientMessage::SwitchRepoExact {
            name: "notes".into(),
            repo_id,
            switch_nonce: Some(SWITCH_NONCE),
        },
    )
    .await?;
    match recv_server_message(ws).await? {
        ServerMessage::RepoSwitched {
            uuid, switch_nonce, ..
        } => assert_eq!((uuid, switch_nonce), (repo_id.to_string(), Some(SWITCH_NONCE))),
        other => panic!("expected RepoSwitched, got {other:?}"),
    }
    let _ = recv_server_message(ws).await?;
    let _ = recv_server_message(ws).await?;
    Ok(())
}

fn assert_protocol_error(
    message: ServerMessage,
    expected_scope: u64,
    expected_code: ServerErrorCode,
    detail: &str,
) {
    match message {
        ServerMessage::ProtocolError {
            error, scope_nonce, ..
        } => {
            assert_eq!(error.code, expected_code);
            assert_eq!(scope_nonce, Some(expected_scope));
            assert!(
                error.detail.as_deref().is_some_and(|got| got.contains(detail)),
                "detail {:?} should contain {detail}",
                error.detail
            );
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
}
