use super::require_browser_switch_nonce;
use crate::server::{channel::DualChannel, session::WsSession};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::{broadcast, mpsc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_switch_nonce_is_required() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(broadcast_tx, uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(9));

    assert!(!require_browser_switch_nonce(
        &ch,
        &session,
        None,
        "branch switch"
    ));

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            scope_nonce,
            switch_nonce,
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(9));
            assert_eq!(switch_nonce, None);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_switch_nonce_must_advance_scope_generation() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(broadcast_tx, uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(9));

    assert!(!require_browser_switch_nonce(
        &ch,
        &session,
        Some(9),
        "branch switch"
    ));

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            scope_nonce,
            switch_nonce,
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(9));
            assert_eq!(switch_nonce, Some(9));
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("stale"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_switch_messages_are_rejected() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(broadcast_tx, uni_tx);
    let session = WsSession::new();

    assert!(!require_browser_switch_nonce(
        &ch,
        &session,
        None,
        "repo switch"
    ));

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            scope_nonce,
            switch_nonce,
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, None);
            assert_eq!(switch_nonce, None);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("browser sessions"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}
