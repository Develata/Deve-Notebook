//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 07_network#server-ws-runtime

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

pub(super) type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(super) async fn ready_writer_ws(
    harness: &WsHarness,
    remote: &IdentityKeyPair,
    scope_nonce: u64,
) -> anyhow::Result<TestWs> {
    let mut ws = connect_harness(harness).await?;
    switch_to_notes_repo(&mut ws, harness.repo_id, scope_nonce).await?;
    send_client_message(
        &mut ws,
        client_sync_hello(remote, harness.repo_id, scope_nonce),
    )
    .await?;
    expect_sync_hello_and_shadow_list(
        &mut ws,
        harness.repo_id,
        scope_nonce,
        &harness.local_peer_id,
        remote,
    )
    .await?;
    send_client_message(
        &mut ws,
        ClientMessage::RegisterWriter {
            peer_id: remote.peer_id(),
            repo_id: harness.repo_id,
            scope_nonce: scope_nonce.into(),
        },
    )
    .await?;
    assert_write_ready(
        recv_server_message(&mut ws).await?,
        harness.repo_id,
        scope_nonce,
        remote,
    );
    Ok(ws)
}

pub(super) async fn create_doc(
    ws: &mut TestWs,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
    path: &str,
) -> anyhow::Result<DocId> {
    send_client_message(
        ws,
        ClientMessage::CreateDoc {
            name: path.into(),
            scope_nonce: Some(scope_nonce),
        },
    )
    .await?;
    let doc_id = assert_create_recovery(
        recv_server_message(ws).await?,
        repo_id,
        scope_nonce,
    );
    send_client_message(
        ws,
        ClientMessage::ListDocs {
            request_id: "create-recovery".into(),
            scope_nonce: Some(scope_nonce),
        },
    )
    .await?;
    let mut saw_doc_list = false;
    let mut saw_tree = false;
    let mut saw_scope = false;
    for _ in 0..3 {
        match recv_server_message(ws).await? {
            ServerMessage::RepoSwitched { uuid, branch, .. } => {
                assert_eq!(uuid, repo_id.to_string());
                assert_eq!(branch, None);
                saw_scope = true;
            }
            ServerMessage::DocList {
                request_id,
                repo_id: Some(actual),
                branch,
                scope_nonce: actual_scope,
                docs,
            } => {
                assert_eq!((actual, branch, actual_scope), (repo_id, None, Some(scope_nonce)));
                assert_eq!(request_id.as_deref(), Some("create-recovery"));
                assert!(docs.iter().any(|(id, candidate)| *id == doc_id && candidate == path));
                saw_doc_list = true;
            }
            ServerMessage::TreeUpdate {
                request_id,
                repo_id: Some(actual),
                branch,
                scope_nonce: actual_scope,
                ..
            } => {
                assert_eq!((actual, branch, actual_scope), (repo_id, None, Some(scope_nonce)));
                assert_eq!(request_id.as_deref(), Some("create-recovery"));
                saw_tree = true;
            }
            other => anyhow::bail!("expected create recovery projection, got {other:?}"),
        }
    }
    anyhow::ensure!(saw_scope && saw_doc_list && saw_tree, "created doc recovery did not settle");
    Ok(doc_id)
}

pub(super) async fn expect_edit_committed(
    ws: &mut TestWs,
    expected: &ExpectedEdit<'_>,
) -> anyhow::Result<u64> {
    let mut new_op_seq = None;
    let mut ack_seq = None;
    for _ in 0..6 {
        match recv_server_message(ws).await? {
            ServerMessage::NewOp {
                repo_id,
                branch,
                scope_nonce,
                doc_id,
                entry,
            } => {
                assert_eq!((repo_id, branch, scope_nonce), expected.scope_tuple());
                assert_eq!(doc_id, expected.doc_id);
                assert_eq!(new_op_seq.replace(entry.seq), None);
                assert_eq!(&entry.op, expected.op);
                assert_origin(entry.origin, expected.client_id, expected.client_op_id);
            }
            ServerMessage::Ack {
                repo_id,
                branch,
                scope_nonce,
                doc_id,
                seq,
                client_op_id,
                ..
            } => {
                assert_eq!((repo_id, branch, scope_nonce), expected.scope_tuple());
                assert_eq!((doc_id, client_op_id), (expected.doc_id, expected.client_op_id));
                assert_eq!(ack_seq.replace(seq), None);
            }
            ServerMessage::EditRejected { error, .. } => {
                anyhow::bail!("edit was rejected after writer registration: {error:?}");
            }
            other => panic!("unexpected edit response before commit ack: {other:?}"),
        }
        if let (Some(new_op_seq), Some(ack_seq)) = (new_op_seq, ack_seq) {
            assert_eq!(ack_seq, new_op_seq);
            return Ok(new_op_seq);
        }
    }
    anyhow::bail!("edit did not emit both NewOp and Ack");
}

pub(super) struct ExpectedEdit<'a> {
    pub(super) repo_id: uuid::Uuid,
    pub(super) scope_nonce: u64,
    pub(super) doc_id: DocId,
    pub(super) op: &'a Op,
    pub(super) client_id: u64,
    pub(super) client_op_id: u64,
}

impl ExpectedEdit<'_> {
    fn scope_tuple(&self) -> (uuid::Uuid, Option<deve_core::models::PeerId>, Option<u64>) {
        (self.repo_id, None, Some(self.scope_nonce))
    }
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
        scope_nonce: hello.scope_nonce.into(),
    }
}

fn assert_write_ready(
    message: ServerMessage,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
    remote: &IdentityKeyPair,
) {
    match message {
        ServerMessage::WriteReady {
            peer_id,
            repo_id: actual,
            scope_nonce: actual_scope,
            branch,
        } => assert_eq!(
            (peer_id, actual, actual_scope, branch),
            (remote.peer_id(), repo_id, scope_nonce.into(), None)
        ),
        other => panic!("expected WriteReady, got {other:?}"),
    }
}

fn assert_origin(
    origin: Option<deve_core::protocol::ClientOrigin>,
    client_id: u64,
    client_op_id: u64,
) {
    let origin = origin.expect("NewOp must preserve client origin");
    assert_eq!((origin.client_id, origin.client_op_id), (client_id, client_op_id));
}

fn assert_create_recovery(
    message: ServerMessage,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> DocId {
    match message {
        ServerMessage::ProjectionRecoveryRequired(required) => {
            assert_eq!(
                (required.repo_id, required.branch, required.scope_nonce),
                (repo_id, None, Some(scope_nonce))
            );
            assert_eq!(
                required.cause,
                deve_core::protocol::ProjectionRecoveryCause::DocumentMutation
            );
            match required.plan.documents {
                deve_core::protocol::DocumentRecoveryScope::Exact(docs) if docs.len() == 1 => {
                    docs[0]
                }
                other => panic!("create recovery must identify one exact document, got {other:?}"),
            }
        }
        other => panic!("expected create projection recovery, got {other:?}"),
    }
}
