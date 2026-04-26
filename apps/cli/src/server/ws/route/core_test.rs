//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Core WebSocket route fail-closed regression coverage.

use super::route_core;
use crate::server::{channel::DualChannel, session::WsSession};
use deve_core::models::DocId;
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[path = "core_test_support.rs"]
mod support;

async fn reject_missing_browser_scope(
    msg: ClientMessage,
    assert_no_extra_response: bool,
) -> anyhow::Result<()> {
    let (_dir, state) = support::build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(7));

    route_core(&state, &ch, &mut session, msg).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(7));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    if assert_no_extra_response {
        assert!(
            uni_rx.try_recv().is_err(),
            "must not continue edit handling"
        );
    }
    Ok(())
}

async fn reject_stale_browser_scope(
    msg: ClientMessage,
    expected_detail: &str,
) -> anyhow::Result<()> {
    let (_dir, state) = support::build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(17));

    route_core(&state, &ch, &mut session, msg).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(16));
            assert!(
                error
                    .detail
                    .as_deref()
                    .expect("detail")
                    .contains(expected_detail)
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(
        uni_rx.try_recv().is_err(),
        "must not continue scoped core handling"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_edit_requires_current_scope_nonce() -> anyhow::Result<()> {
    reject_missing_browser_scope(
        ClientMessage::Edit {
            doc_id: DocId(uuid::Uuid::new_v4()),
            op: deve_core::models::Op::Insert {
                pos: 0,
                content: "x".into(),
            },
            client_id: 1,
            client_op_id: 2,
            scope_nonce: None,
        },
        true,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_edit_rejects_stale_scope_before_handler() -> anyhow::Result<()> {
    reject_stale_browser_scope(
        ClientMessage::Edit {
            doc_id: DocId(uuid::Uuid::new_v4()),
            op: deve_core::models::Op::Insert {
                pos: 0,
                content: "x".into(),
            },
            client_id: 1,
            client_op_id: 2,
            scope_nonce: Some(16),
        },
        "edit scope nonce is stale",
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_open_doc_requires_current_scope_nonce() -> anyhow::Result<()> {
    reject_missing_browser_scope(
        ClientMessage::OpenDoc {
            doc_id: DocId(uuid::Uuid::new_v4()),
            request_id: 1,
            scope_nonce: None,
        },
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_request_history_requires_current_scope_nonce() -> anyhow::Result<()> {
    reject_missing_browser_scope(
        ClientMessage::RequestHistory {
            doc_id: DocId(uuid::Uuid::new_v4()),
            request_id: 1,
            scope_nonce: None,
        },
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_request_key_requires_current_scope_nonce() -> anyhow::Result<()> {
    reject_missing_browser_scope(ClientMessage::RequestKey { scope_nonce: None }, false).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_search_requires_current_scope_nonce() -> anyhow::Result<()> {
    reject_missing_browser_scope(
        ClientMessage::Search {
            request_id: "search-1".into(),
            query: "abc".into(),
            limit: 10,
            scope_nonce: None,
        },
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_delete_peer_requires_current_scope_nonce() -> anyhow::Result<()> {
    reject_missing_browser_scope(
        ClientMessage::DeletePeer {
            peer_id: "peer-a".into(),
            scope_nonce: None,
        },
        false,
    )
    .await
}
