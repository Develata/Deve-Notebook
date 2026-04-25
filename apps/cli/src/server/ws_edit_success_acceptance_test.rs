//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 05_network#server-ws-runtime

use super::sync_hello_test_support::signed_hello_for_scope;
use super::ws_protocol_acceptance_support::{
    WsHarness, connect_harness, expect_sync_hello_and_shadow_list, recv_server_message,
    send_client_message, switch_to_notes_repo,
};
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::security::IdentityKeyPair;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const SCOPE: u64 = 1;
const CLIENT_ID: u64 = 13;
const CLIENT_OP_ID: u64 = 17;
type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_edit_after_register_writer_emits_new_op_and_ack() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let remote = IdentityKeyPair::generate();
    let mut ws = ready_writer_ws(&harness, &remote).await?;
    let doc_id = create_doc(&mut ws, harness.repo_id).await?;

    send_client_message(
        &mut ws,
        ClientMessage::Edit {
            doc_id,
            op: inserted_op(),
            client_id: CLIENT_ID,
            client_op_id: CLIENT_OP_ID,
            scope_nonce: Some(SCOPE),
        },
    )
    .await?;
    expect_edit_committed(&mut ws, harness.repo_id, doc_id).await?;

    harness.shutdown().await;
    Ok(())
}

async fn ready_writer_ws(
    harness: &WsHarness,
    remote: &IdentityKeyPair,
) -> anyhow::Result<TestWs> {
    let mut ws = connect_harness(harness).await?;
    switch_to_notes_repo(&mut ws, harness.repo_id, SCOPE).await?;
    send_client_message(&mut ws, client_sync_hello(remote, harness.repo_id)).await?;
    expect_sync_hello_and_shadow_list(
        &mut ws,
        harness.repo_id,
        SCOPE,
        &harness.local_peer_id,
        remote,
    )
    .await?;
    send_client_message(
        &mut ws,
        ClientMessage::RegisterWriter {
            peer_id: remote.peer_id(),
            repo_id: harness.repo_id,
            scope_nonce: SCOPE,
        },
    )
    .await?;
    assert_write_ready(recv_server_message(&mut ws).await?, harness.repo_id, remote);
    Ok(ws)
}

async fn create_doc(ws: &mut TestWs, repo_id: uuid::Uuid) -> anyhow::Result<DocId> {
    send_client_message(
        ws,
        ClientMessage::CreateDoc {
            name: "writer-success.md".into(),
            scope_nonce: Some(SCOPE),
        },
    )
    .await?;
    let mut doc_id = None;
    let mut saw_tree = false;
    for _ in 0..4 {
        track_create_message(recv_server_message(ws).await?, repo_id, &mut doc_id, &mut saw_tree);
        if doc_id.is_some() && saw_tree {
            break;
        }
    }
    drain_create_tail(ws, repo_id).await?;
    doc_id.filter(|_| saw_tree)
        .ok_or_else(|| anyhow::anyhow!("created doc view did not settle"))
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

async fn expect_edit_committed(
    ws: &mut TestWs,
    repo_id: uuid::Uuid,
    doc_id: DocId,
) -> anyhow::Result<()> {
    let mut new_op_seq = None;
    let mut ack_seq = None;
    for _ in 0..6 {
        match recv_server_message(ws).await? {
            ServerMessage::NewOp {
                repo_id: actual,
                branch,
                scope_nonce,
                doc_id: actual_doc,
                entry,
            } => {
                assert_eq!((actual, branch, scope_nonce), (repo_id, None, Some(SCOPE)));
                assert_eq!(actual_doc, doc_id);
                assert_eq!(new_op_seq.replace(entry.seq), None);
                assert_new_op(entry.origin);
                assert_eq!(entry.op, inserted_op());
            }
            ServerMessage::Ack {
                repo_id: actual,
                branch,
                scope_nonce,
                doc_id: actual_doc,
                seq,
                client_op_id,
                ..
            } => {
                assert_eq!((actual, branch, scope_nonce), (repo_id, None, Some(SCOPE)));
                assert_eq!((actual_doc, client_op_id), (doc_id, CLIENT_OP_ID));
                assert_eq!(ack_seq.replace(seq), None);
            }
            ServerMessage::EditRejected { error, .. } => {
                anyhow::bail!("edit was rejected after writer registration: {error:?}");
            }
            other => panic!("unexpected edit response before commit ack: {other:?}"),
        }
        if let (Some(new_op_seq), Some(ack_seq)) = (new_op_seq, ack_seq) {
            assert_eq!(ack_seq, new_op_seq);
            return Ok(());
        }
    }
    anyhow::bail!("edit did not emit both NewOp and Ack");
}

fn assert_new_op(origin: Option<deve_core::protocol::ClientOrigin>) {
    let origin = origin.expect("NewOp must preserve client origin");
    assert_eq!(origin.client_id, CLIENT_ID);
    assert_eq!(origin.client_op_id, CLIENT_OP_ID);
}

fn inserted_op() -> Op {
    Op::Insert {
        pos: 0,
        content: "ok".into(),
    }
}

fn assert_write_ready(message: ServerMessage, repo_id: uuid::Uuid, remote: &IdentityKeyPair) {
    match message {
        ServerMessage::WriteReady {
            peer_id,
            repo_id: actual,
            scope_nonce,
            branch,
        } => assert_eq!(
            (peer_id, actual, scope_nonce, branch),
            (remote.peer_id(), repo_id, SCOPE, None)
        ),
        other => panic!("expected WriteReady, got {other:?}"),
    }
}

fn track_create_message(
    message: ServerMessage,
    repo_id: uuid::Uuid,
    doc_id: &mut Option<DocId>,
    saw_tree: &mut bool,
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
            *doc_id = docs
                .into_iter()
                .find(|(_id, path)| path == "writer-success.md")
                .map(|(id, _path)| id);
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
            *saw_tree = true;
        }
        ServerMessage::FsChangeDetected { .. } => {}
        other => panic!("expected create-doc view message, got {other:?}"),
    }
}

async fn drain_create_tail(ws: &mut TestWs, repo_id: uuid::Uuid) -> anyhow::Result<()> {
    match recv_server_message(ws).await? {
        ServerMessage::FsChangeDetected {
            repo_id: Some(actual),
            path,
            change_type,
            ..
        } => {
            assert_eq!(actual, repo_id);
            assert_eq!((path.as_str(), change_type.as_str()), ("writer-success.md", "added"));
        }
        other => panic!("unexpected create-doc tail message, got {other:?}"),
    }
    Ok(())
}
