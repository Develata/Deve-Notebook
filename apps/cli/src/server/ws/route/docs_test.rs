use super::{requested_scope_nonce, route_docs};
use crate::server::sync_hello_test_support::{build_state, unicast_channel};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use tokio::time::{Duration, timeout};

#[test]
fn extracts_scope_nonce_from_doc_messages() {
    let cases = [
        ClientMessage::CreateDoc {
            name: "notes".into(),
            scope_nonce: Some(3),
        },
        ClientMessage::RenameDoc {
            old_path: "a.md".into(),
            new_path: "b.md".into(),
            scope_nonce: Some(3),
        },
        ClientMessage::DeleteDoc {
            path: "a.md".into(),
            scope_nonce: Some(3),
        },
        ClientMessage::CopyDoc {
            src_path: "a.md".into(),
            dest_path: "b.md".into(),
            scope_nonce: Some(3),
        },
        ClientMessage::MoveDoc {
            src_path: "a.md".into(),
            dest_path: "b.md".into(),
            scope_nonce: Some(3),
        },
    ];
    for msg in cases {
        assert_eq!(requested_scope_nonce(&msg), Some(Some(3)));
    }
    assert_eq!(requested_scope_nonce(&ClientMessage::Ping), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_doc_rejects_stale_scope_before_handler() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = crate::server::session::WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(17));

    route_docs(
        &state,
        &ch,
        &mut session,
        ClientMessage::DeleteDoc {
            path: "notes/a.md".into(),
            scope_nonce: Some(16),
        },
    )
    .await;

    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("protocol error")
    {
        ServerMessage::ProtocolError {
            error, scope_nonce, ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(16));
            assert!(
                error
                    .detail
                    .as_deref()
                    .expect("detail")
                    .contains("document scope nonce is stale")
            );
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn docs_scope_nonce_gate_rejects_missing_scope_before_handler() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    for msg in doc_messages_with_scope(None) {
        route_docs(&state, &ch, &mut session, msg).await;
        assert_scope_guard_error(
            recv_unicast_message(&mut uni_rx).await?,
            Some(17),
            "document scope nonce missing",
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn docs_scope_nonce_gate_rejects_stale_scope_before_handler() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    for msg in doc_messages_with_scope(Some(16)) {
        route_docs(&state, &ch, &mut session, msg).await;
        assert_scope_guard_error(
            recv_unicast_message(&mut uni_rx).await?,
            Some(16),
            "document scope nonce is stale",
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_doc_message_falls_through_to_core_route() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    route_docs(&state, &ch, &mut session, ClientMessage::Ping).await;

    match recv_unicast_message(&mut uni_rx).await? {
        ServerMessage::Pong => {}
        other => panic!("expected Pong, got {other:?}"),
    }
    Ok(())
}

fn browser_session(scope_nonce: u64) -> crate::server::session::WsSession {
    let mut session = crate::server::session::WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

fn doc_messages_with_scope(scope_nonce: Option<u64>) -> Vec<ClientMessage> {
    vec![
        ClientMessage::CreateDoc {
            name: "notes/a.md".into(),
            scope_nonce,
        },
        ClientMessage::RenameDoc {
            old_path: "notes/a.md".into(),
            new_path: "notes/b.md".into(),
            scope_nonce,
        },
        ClientMessage::DeleteDoc {
            path: "notes/a.md".into(),
            scope_nonce,
        },
        ClientMessage::CopyDoc {
            src_path: "notes/a.md".into(),
            dest_path: "notes/c.md".into(),
            scope_nonce,
        },
        ClientMessage::MoveDoc {
            src_path: "notes/a.md".into(),
            dest_path: "notes/d.md".into(),
            scope_nonce,
        },
    ]
}

async fn recv_unicast_message(
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
