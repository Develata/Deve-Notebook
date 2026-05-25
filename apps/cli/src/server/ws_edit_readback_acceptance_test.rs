//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 07_network#server-ws-runtime

use super::ws_edit_flow_acceptance_support::{
    ExpectedEdit, TestWs, create_doc, expect_edit_committed, ready_writer_ws,
};
use super::ws_protocol_acceptance_support::{WsHarness, recv_server_message, send_client_message};
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ClientMessage, ConfirmedOp, ServerMessage};
use deve_core::security::IdentityKeyPair;

const SCOPE: u64 = 1;
const CLIENT_ID: u64 = 23;
const FIRST_CLIENT_OP_ID: u64 = 29;
const SECOND_CLIENT_OP_ID: u64 = 31;
const FIRST_OPEN_REQUEST_ID: u64 = 37;
const SECOND_OPEN_REQUEST_ID: u64 = 41;
const HISTORY_REQUEST_ID: u64 = 43;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_open_doc_and_history_read_back_registered_edit() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let remote = IdentityKeyPair::generate();
    let mut ws = ready_writer_ws(&harness, &remote, SCOPE).await?;
    let doc_id = create_doc(&mut ws, harness.repo_id, SCOPE, "writer-readback.md").await?;
    let first_op = inserted_op();
    send_edit(&mut ws, doc_id, first_op.clone(), FIRST_CLIENT_OP_ID).await?;
    let first_seq = expect_edit_committed(
        &mut ws,
        &ExpectedEdit {
            repo_id: harness.repo_id,
            scope_nonce: SCOPE,
            doc_id,
            op: &first_op,
            client_id: CLIENT_ID,
            client_op_id: FIRST_CLIENT_OP_ID,
        },
    )
    .await?;

    request_open_doc(&mut ws, doc_id, FIRST_OPEN_REQUEST_ID).await?;
    assert_snapshot(
        recv_server_message(&mut ws).await?,
        harness.repo_id,
        doc_id,
        SnapshotExpect {
            request_id: FIRST_OPEN_REQUEST_ID,
            min_version: first_seq,
            content: "readback",
            delta: None,
        },
    );

    let second_op = delta_op();
    send_edit(&mut ws, doc_id, second_op.clone(), SECOND_CLIENT_OP_ID).await?;
    let second_seq = expect_edit_committed(
        &mut ws,
        &ExpectedEdit {
            repo_id: harness.repo_id,
            scope_nonce: SCOPE,
            doc_id,
            op: &second_op,
            client_id: CLIENT_ID,
            client_op_id: SECOND_CLIENT_OP_ID,
        },
    )
    .await?;

    request_open_doc(&mut ws, doc_id, SECOND_OPEN_REQUEST_ID).await?;
    assert_snapshot(
        recv_server_message(&mut ws).await?,
        harness.repo_id,
        doc_id,
        SnapshotExpect {
            request_id: SECOND_OPEN_REQUEST_ID,
            min_version: second_seq,
            content: "readback-delta",
            delta: Some((&second_op, SECOND_CLIENT_OP_ID)),
        },
    );

    request_history(&mut ws, doc_id).await?;
    assert_history(
        recv_server_message(&mut ws).await?,
        harness.repo_id,
        doc_id,
        &second_op,
        SECOND_CLIENT_OP_ID,
    );

    harness.shutdown().await;
    Ok(())
}

async fn send_edit(ws: &mut TestWs, doc_id: DocId, op: Op, client_op_id: u64) -> anyhow::Result<()> {
    send_client_message(
        ws,
        ClientMessage::Edit {
            doc_id,
            op,
            client_id: CLIENT_ID,
            client_op_id,
            scope_nonce: Some(SCOPE),
        },
    )
    .await
}

async fn request_open_doc(ws: &mut TestWs, doc_id: DocId, request_id: u64) -> anyhow::Result<()> {
    send_client_message(
        ws,
        ClientMessage::OpenDoc {
            doc_id,
            request_id,
            scope_nonce: Some(SCOPE),
        },
    )
    .await
}

async fn request_history(ws: &mut TestWs, doc_id: DocId) -> anyhow::Result<()> {
    send_client_message(
        ws,
        ClientMessage::RequestHistory {
            doc_id,
            request_id: HISTORY_REQUEST_ID,
            scope_nonce: Some(SCOPE),
        },
    )
    .await
}

struct SnapshotExpect<'a> {
    request_id: u64,
    min_version: u64,
    content: &'a str,
    delta: Option<(&'a Op, u64)>,
}

fn assert_snapshot(
    message: ServerMessage,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    expected: SnapshotExpect<'_>,
) {
    match message {
        ServerMessage::Snapshot {
            repo_id: actual,
            branch,
            scope_nonce,
            doc_id: actual_doc,
            request_id,
            content,
            base_seq,
            version,
            delta_ops,
        } => {
            assert_eq!((actual, branch, scope_nonce), (repo_id, None, Some(SCOPE)));
            assert_eq!((actual_doc, request_id), (doc_id, expected.request_id));
            assert!(version >= expected.min_version);
            assert_eq!(reconstruct_snapshot_content(content, &delta_ops), expected.content);
            assert!(base_seq <= version);
            match expected.delta {
                Some((op, client_op_id)) => assert_delta_origin(&delta_ops, op, client_op_id),
                None => assert!(delta_ops.is_empty()),
            }
        }
        other => panic!("expected Snapshot, got {other:?}"),
    }
}

fn assert_history(message: ServerMessage, repo_id: uuid::Uuid, doc_id: DocId, op: &Op, client_op_id: u64) {
    match message {
        ServerMessage::History {
            repo_id: actual,
            branch,
            scope_nonce,
            doc_id: actual_doc,
            request_id,
            ops,
        } => {
            assert_eq!((actual, branch, scope_nonce), (repo_id, None, Some(SCOPE)));
            assert_eq!((actual_doc, request_id), (doc_id, HISTORY_REQUEST_ID));
            let matches = matching_origin_ops(&ops, op, client_op_id);
            assert_eq!(matches.len(), 1, "history must include one matching edit");
            assert!(matches[0].seq > 0);
            assert_origin(matches[0], client_op_id);
        }
        other => panic!("expected History, got {other:?}"),
    }
}

fn assert_delta_origin(delta_ops: &[ConfirmedOp], op: &Op, client_op_id: u64) {
    let matches = matching_origin_ops(delta_ops, op, client_op_id);
    assert_eq!(matches.len(), 1, "snapshot delta must include one matching edit");
    assert!(matches[0].seq > 0);
    assert_origin(matches[0], client_op_id);
}

fn matching_origin_ops<'a>(ops: &'a [ConfirmedOp], op: &Op, client_op_id: u64) -> Vec<&'a ConfirmedOp> {
    ops.iter()
        .filter(|entry| &entry.op == op && has_client_origin(entry, client_op_id))
        .collect()
}

fn reconstruct_snapshot_content(content: String, delta_ops: &[ConfirmedOp]) -> String {
    let mut text = content;
    for entry in delta_ops {
        apply_op(&mut text, &entry.op);
    }
    text
}

fn apply_op(text: &mut String, op: &Op) {
    match op {
        Op::Insert { pos, content } => text.insert_str(*pos as usize, content),
        Op::Delete { pos, len } => {
            let start = *pos as usize;
            text.replace_range(start..start + *len as usize, "");
        }
    }
}

fn assert_origin(entry: &ConfirmedOp, client_op_id: u64) {
    let origin = entry.origin.expect("entry must preserve client origin");
    assert_eq!((origin.client_id, origin.client_op_id), (CLIENT_ID, client_op_id));
}

fn has_client_origin(entry: &ConfirmedOp, client_op_id: u64) -> bool {
    entry
        .origin
        .is_some_and(|origin| (origin.client_id, origin.client_op_id) == (CLIENT_ID, client_op_id))
}

fn inserted_op() -> Op {
    Op::Insert {
        pos: 0,
        content: "readback".into(),
    }
}

fn delta_op() -> Op {
    Op::Insert {
        pos: 8,
        content: "-delta".into(),
    }
}
