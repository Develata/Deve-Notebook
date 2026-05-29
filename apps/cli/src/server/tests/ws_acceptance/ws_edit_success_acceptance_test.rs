//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 07_network#server-ws-runtime

use super::ws_edit_flow_acceptance_support::{
    ExpectedEdit, create_doc, expect_edit_committed, ready_writer_ws,
};
use super::ws_protocol_acceptance_support::{WsHarness, send_client_message};
use deve_core::models::Op;
use deve_core::protocol::ClientMessage;
use deve_core::security::IdentityKeyPair;

const SCOPE: u64 = 1;
const CLIENT_ID: u64 = 13;
const CLIENT_OP_ID: u64 = 17;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_edit_after_register_writer_emits_new_op_and_ack() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let remote = IdentityKeyPair::generate();
    let mut ws = ready_writer_ws(&harness, &remote, SCOPE).await?;
    let doc_id = create_doc(&mut ws, harness.repo_id, SCOPE, "writer-success.md").await?;
    let op = inserted_op();

    send_client_message(
        &mut ws,
        ClientMessage::Edit {
            doc_id,
            op: op.clone(),
            client_id: CLIENT_ID,
            client_op_id: CLIENT_OP_ID,
            scope_nonce: Some(SCOPE),
        },
    )
    .await?;
    expect_edit_committed(
        &mut ws,
        &ExpectedEdit {
            repo_id: harness.repo_id,
            scope_nonce: SCOPE,
            doc_id,
            op: &op,
            client_id: CLIENT_ID,
            client_op_id: CLIENT_OP_ID,
        },
    )
    .await?;

    harness.shutdown().await;
    Ok(())
}

fn inserted_op() -> Op {
    Op::Insert {
        pos: 0,
        content: "ok".into(),
    }
}
