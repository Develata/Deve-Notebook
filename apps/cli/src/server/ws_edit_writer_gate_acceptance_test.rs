//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 05_network#server-ws-runtime

use super::sync_hello_test_support::signed_hello_for_scope;
use super::ws_protocol_acceptance_support::{
    WsHarness, connect_harness, expect_sync_hello_and_shadow_list, recv_optional_server_message,
    recv_server_message, send_client_message, switch_to_notes_repo,
};
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use deve_core::security::IdentityKeyPair;
use tokio::net::TcpStream;
use tokio::time::Duration;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const SCOPE: u64 = 1;
const CLIENT_ID: u64 = 7;
const CLIENT_OP_ID: u64 = 9;
type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_edit_after_sync_hello_requires_registered_writer() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let mut ws = ready_ws(&harness).await?;
    let doc_id = create_doc(&mut ws, harness.repo_id).await?;

    send_client_message(
        &mut ws,
        ClientMessage::Edit {
            doc_id,
            op: Op::Insert {
                pos: 0,
                content: "x".into(),
            },
            client_id: CLIENT_ID,
            client_op_id: CLIENT_OP_ID,
            scope_nonce: Some(SCOPE),
        },
    )
    .await?;
    assert_writer_rejected(recv_until_edit_rejected(&mut ws, doc_id).await?, doc_id);
    assert_no_commit_echo(&mut ws, doc_id).await?;

    harness.shutdown().await;
    Ok(())
}

async fn ready_ws(harness: &WsHarness) -> anyhow::Result<TestWs> {
    let remote = IdentityKeyPair::generate();
    let mut ws = connect_harness(harness).await?;
    switch_to_notes_repo(&mut ws, harness.repo_id, SCOPE).await?;
    send_client_message(&mut ws, client_sync_hello(&remote, harness.repo_id)).await?;
    expect_sync_hello_and_shadow_list(
        &mut ws,
        harness.repo_id,
        SCOPE,
        &harness.local_peer_id,
        &remote,
    )
    .await?;
    Ok(ws)
}

async fn create_doc(ws: &mut TestWs, repo_id: uuid::Uuid) -> anyhow::Result<DocId> {
    send_client_message(
        ws,
        ClientMessage::CreateDoc {
            name: "writer-gate.md".into(),
            scope_nonce: Some(SCOPE),
        },
    )
    .await?;
    let mut created_doc = None;
    let mut saw_tree_update = false;
    for _ in 0..4 {
        track_create_view_message(
            recv_server_message(ws).await?,
            repo_id,
            &mut created_doc,
            &mut saw_tree_update,
        );
        if created_doc.is_some() && saw_tree_update {
            break;
        }
    }
    let Some(doc_id) = created_doc else {
        anyhow::bail!("created doc did not appear in DocList");
    };
    if !saw_tree_update {
        anyhow::bail!("created doc did not produce a TreeUpdate");
    }
    drain_create_tail(ws, repo_id).await?;
    Ok(doc_id)
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

async fn recv_until_edit_rejected(
    ws: &mut TestWs,
    expected_doc: DocId,
) -> anyhow::Result<ServerMessage> {
    for _ in 0..6 {
        let msg = recv_server_message(ws).await?;
        reject_commit_echo(&msg, expected_doc)?;
        if matches!(msg, ServerMessage::EditRejected { .. }) {
            return Ok(msg);
        }
    }
    anyhow::bail!("expected EditRejected");
}

async fn assert_no_commit_echo(ws: &mut TestWs, expected_doc: DocId) -> anyhow::Result<()> {
    for _ in 0..6 {
        let Some(msg) = recv_optional_server_message(ws, Duration::from_millis(120)).await? else {
            return Ok(());
        };
        reject_commit_echo(&msg, expected_doc)?;
    }
    anyhow::bail!("server kept emitting messages after rejected edit");
}

async fn drain_create_tail(ws: &mut TestWs, repo_id: uuid::Uuid) -> anyhow::Result<()> {
    for _ in 0..4 {
        let Some(msg) = recv_optional_server_message(ws, Duration::from_millis(120)).await? else {
            return Ok(());
        };
        assert_create_tail_message(msg, repo_id);
    }
    anyhow::bail!("create doc emitted too many tail messages");
}

fn reject_commit_echo(message: &ServerMessage, expected_doc: DocId) -> anyhow::Result<()> {
    if is_commit_echo(message, expected_doc) {
        anyhow::bail!("unauthorized edit was committed: {message:?}");
    }
    Ok(())
}

fn assert_writer_rejected(message: ServerMessage, expected_doc: DocId) {
    match message {
        ServerMessage::EditRejected {
            scope_nonce,
            doc_id,
            client_op_id,
            error,
        } => {
            assert_eq!(scope_nonce, Some(SCOPE));
            assert_eq!(doc_id, expected_doc);
            assert_eq!(client_op_id, CLIENT_OP_ID);
            assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
            assert_eq!(error.detail, None);
        }
        other => panic!("expected EditRejected, got {other:?}"),
    }
}

fn is_commit_echo(message: &ServerMessage, expected_doc: DocId) -> bool {
    match message {
        ServerMessage::Ack {
            doc_id,
            client_op_id,
            ..
        } => *doc_id == expected_doc && *client_op_id == CLIENT_OP_ID,
        ServerMessage::NewOp { doc_id, entry, .. } => {
            *doc_id == expected_doc
                && entry
                    .origin
                    .as_ref()
                    .is_some_and(|origin| {
                        origin.client_id == CLIENT_ID && origin.client_op_id == CLIENT_OP_ID
                    })
        }
        _ => false,
    }
}

fn track_create_view_message(
    message: ServerMessage,
    repo_id: uuid::Uuid,
    created_doc: &mut Option<DocId>,
    saw_tree_update: &mut bool,
) {
    match message {
        ServerMessage::DocList {
            repo_id: Some(actual),
            branch,
            scope_nonce,
            docs,
            ..
        } => {
            assert_eq!(actual, repo_id);
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(SCOPE));
            if let Some((doc_id, _path)) =
                docs.into_iter().find(|(_id, path)| path == "writer-gate.md")
            {
                *created_doc = Some(doc_id);
            }
        }
        ServerMessage::TreeUpdate {
            repo_id: Some(actual),
            branch,
            scope_nonce,
            ..
        } => {
            assert_eq!(actual, repo_id);
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(SCOPE));
            *saw_tree_update = true;
        }
        ServerMessage::FsChangeDetected { .. } => {}
        other => panic!("expected create-doc view message, got {other:?}"),
    }
}

fn assert_create_tail_message(message: ServerMessage, repo_id: uuid::Uuid) {
    match message {
        ServerMessage::FsChangeDetected {
            repo_id: Some(actual),
            path,
            change_type,
            ..
        } => {
            assert_eq!(actual, repo_id);
            assert_eq!(path, "writer-gate.md");
            assert_eq!(change_type, "added");
        }
        other => panic!("unexpected create-doc tail message, got {other:?}"),
    }
}
