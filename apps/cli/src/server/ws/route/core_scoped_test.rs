use super::route_scoped_core;
use crate::server::sync_hello_test_support::{build_state, unicast_channel};
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use tokio::time::{Duration, timeout};

#[test]
fn extracts_scope_nonce_from_core_scoped_messages() {
    let doc_id = DocId::new();
    let cases = [
        (
            ClientMessage::OpenDoc {
                doc_id,
                request_id: 1,
                scope_nonce: Some(11),
            },
            "open doc",
        ),
        (
            ClientMessage::RequestHistory {
                doc_id,
                request_id: 2,
                scope_nonce: Some(11),
            },
            "document history",
        ),
        (
            ClientMessage::Edit {
                doc_id,
                op: insert_op("x"),
                client_id: 1,
                client_op_id: 2,
                scope_nonce: Some(11),
            },
            "edit",
        ),
        (
            ClientMessage::ListDocs {
                request_id: "docs".into(),
                scope_nonce: Some(11),
            },
            "document list",
        ),
        (
            ClientMessage::ListShadows {
                request_id: "shadows".into(),
                scope_nonce: Some(11),
            },
            "shadow list",
        ),
        (
            ClientMessage::ListRepos {
                request_id: "repos".into(),
                scope_nonce: Some(11),
            },
            "repo list",
        ),
        (
            ClientMessage::Search {
                request_id: "search".into(),
                query: "abc".into(),
                limit: 10,
                scope_nonce: Some(11),
            },
            "search",
        ),
        (
            ClientMessage::DeletePeer {
                peer_id: "peer-a".into(),
                scope_nonce: Some(11),
            },
            "delete peer",
        ),
        (
            ClientMessage::RequestKey {
                scope_nonce: Some(11),
            },
            "request key",
        ),
    ];
    for (msg, scope_name) in cases {
        let gate = msg.core_scope_gate().expect("scope gate");
        assert_eq!((gate.scope_nonce, gate.scope_name), (Some(11), scope_name));
    }
    assert_eq!(ClientMessage::Ping.core_scope_gate(), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn core_scoped_scope_nonce_gate_rejects_missing_scope_before_handler() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    for msg in core_scoped_messages_with_scope(None) {
        let routed = route_scoped_core(&state, &ch, &mut session, msg).await;
        assert!(routed.is_none());
        assert_scope_guard_error(
            recv_protocol_error(&mut uni_rx).await?,
            Some(17),
            "scope nonce missing",
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn core_scoped_scope_nonce_gate_rejects_stale_scope_before_handler() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    for msg in core_scoped_messages_with_scope(Some(16)) {
        let routed = route_scoped_core(&state, &ch, &mut session, msg).await;
        assert!(routed.is_none());
        assert_scope_guard_error(
            recv_protocol_error(&mut uni_rx).await?,
            Some(16),
            "scope nonce is stale",
        );
    }
    Ok(())
}

fn browser_session(scope_nonce: u64) -> crate::server::session::WsSession {
    let mut session = crate::server::session::WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

fn core_scoped_messages_with_scope(scope_nonce: Option<u64>) -> Vec<ClientMessage> {
    let doc_id = DocId::new();
    vec![
        ClientMessage::OpenDoc {
            doc_id,
            request_id: 1,
            scope_nonce,
        },
        ClientMessage::RequestHistory {
            doc_id,
            request_id: 2,
            scope_nonce,
        },
        ClientMessage::Edit {
            doc_id,
            op: insert_op("x"),
            client_id: 1,
            client_op_id: 2,
            scope_nonce,
        },
        ClientMessage::ListDocs {
            request_id: "docs".into(),
            scope_nonce,
        },
        ClientMessage::ListShadows {
            request_id: "shadows".into(),
            scope_nonce,
        },
        ClientMessage::ListRepos {
            request_id: "repos".into(),
            scope_nonce,
        },
        ClientMessage::Search {
            request_id: "search".into(),
            query: "abc".into(),
            limit: 10,
            scope_nonce,
        },
        ClientMessage::DeletePeer {
            peer_id: "peer-a".into(),
            scope_nonce,
        },
        ClientMessage::RequestKey { scope_nonce },
    ]
}

fn insert_op(content: &str) -> Op {
    Op::Insert {
        pos: 0,
        content: content.into(),
    }
}

async fn recv_protocol_error(
    uni_rx: &mut tokio::sync::mpsc::Receiver<ServerMessage>,
) -> anyhow::Result<ServerMessage> {
    Ok(timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("protocol error"))
}

fn assert_scope_guard_error(message: ServerMessage, scope_nonce: Option<u64>, detail: &str) {
    match message {
        ServerMessage::ProtocolError {
            error,
            scope_nonce: actual_scope_nonce,
            ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(actual_scope_nonce, scope_nonce);
            assert!(error.detail.as_deref().expect("detail").contains(detail));
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
}
