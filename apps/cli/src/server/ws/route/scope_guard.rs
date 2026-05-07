//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Browser scope nonce guard for repo-scoped WebSocket routes.

use crate::server::{channel::DualChannel, session::WsSession};
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) fn validate_browser_scope_nonce(
    session: &WsSession,
    requested_scope_nonce: Option<u64>,
    scope_name: &str,
) -> Result<(), ServerError> {
    if !session.is_browser_session() {
        return Ok(());
    }
    let Some(requested_scope_nonce) = requested_scope_nonce else {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!("{scope_name} scope nonce missing"),
        ));
    };
    if session.scope_nonce() != requested_scope_nonce {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScStaleScope,
            format!(
                "{scope_name} scope nonce is stale: current_scope_nonce={}, requested_scope_nonce={}",
                session.scope_nonce(),
                requested_scope_nonce
            ),
        ));
    }
    Ok(())
}

pub(super) fn response_scope_nonce(
    session: &WsSession,
    requested_scope_nonce: Option<u64>,
) -> Option<u64> {
    if !session.is_browser_session() {
        return requested_scope_nonce;
    }
    requested_scope_nonce.or(Some(session.scope_nonce()))
}

pub(super) fn reject_invalid_browser_scope_nonce(
    ch: &DualChannel,
    session: &WsSession,
    requested_scope_nonce: Option<u64>,
    scope_name: &str,
) -> bool {
    match validate_browser_scope_nonce(session, requested_scope_nonce, scope_name) {
        Ok(()) => false,
        Err(error) => {
            ch.send_protocol_error_with_scope_nonce(
                error,
                response_scope_nonce(session, requested_scope_nonce),
            );
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        reject_invalid_browser_scope_nonce, response_scope_nonce, validate_browser_scope_nonce,
    };
    use crate::server::{channel::DualChannel, session::WsSession};
    use deve_core::protocol::{ServerErrorCode, ServerMessage};
    use tokio::sync::{broadcast, mpsc};

    #[test]
    fn browser_scope_guard_requires_current_scope_nonce() {
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(11));
        let missing = validate_browser_scope_nonce(&session, None, "merge").unwrap_err();
        assert_eq!(missing.code, ServerErrorCode::ScRepoContextInvalid);
        let stale = validate_browser_scope_nonce(&session, Some(10), "merge").unwrap_err();
        assert_eq!(stale.code, ServerErrorCode::ScStaleScope);
        assert!(stale.detail.as_deref().expect("detail").contains("stale"));
        assert!(validate_browser_scope_nonce(&session, Some(11), "merge").is_ok());
    }

    #[test]
    fn non_browser_scope_guard_allows_missing_nonce() {
        let session = WsSession::new();
        assert!(validate_browser_scope_nonce(&session, None, "source control").is_ok());
    }

    #[test]
    fn browser_scope_guard_response_nonce_uses_current_scope_for_missing_requests() {
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(11));
        assert_eq!(response_scope_nonce(&session, None), Some(11));
        assert_eq!(response_scope_nonce(&session, Some(7)), Some(7));
    }

    #[test]
    fn browser_scope_guard_rejects_and_sends_structured_error() {
        let (ch, mut unicast_rx) = test_channel();
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(11));

        assert!(reject_invalid_browser_scope_nonce(
            &ch,
            &session,
            Some(10),
            "source control"
        ));

        match unicast_rx.try_recv().expect("protocol error") {
            ServerMessage::ProtocolError {
                error, scope_nonce, ..
            } => {
                assert_eq!(error.code, ServerErrorCode::ScStaleScope);
                assert_eq!(scope_nonce, Some(10));
                assert!(
                    error
                        .detail
                        .as_deref()
                        .expect("detail")
                        .contains("source control scope nonce is stale")
                );
            }
            other => panic!("expected ProtocolError, got {other:?}"),
        }
    }

    #[test]
    fn browser_scope_guard_missing_nonce_reports_current_scope_nonce() {
        let (ch, mut unicast_rx) = test_channel();
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(11));

        assert!(reject_invalid_browser_scope_nonce(
            &ch,
            &session,
            None,
            "source control"
        ));

        match unicast_rx.try_recv().expect("protocol error") {
            ServerMessage::ProtocolError {
                error, scope_nonce, ..
            } => {
                assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
                assert_eq!(scope_nonce, Some(11));
                assert!(
                    error
                        .detail
                        .as_deref()
                        .expect("detail")
                        .contains("source control scope nonce missing")
                );
            }
            other => panic!("expected ProtocolError, got {other:?}"),
        }
    }

    #[test]
    fn non_browser_scope_guard_does_not_send_error() {
        let (ch, mut unicast_rx) = test_channel();
        let session = WsSession::new();

        assert!(!reject_invalid_browser_scope_nonce(
            &ch,
            &session,
            None,
            "source control"
        ));
        assert!(matches!(
            unicast_rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    fn test_channel() -> (DualChannel, mpsc::Receiver<ServerMessage>) {
        let (broadcast_tx, _broadcast_rx) = broadcast::channel(8);
        let (unicast_tx, unicast_rx) = mpsc::channel(8);
        (DualChannel::new(broadcast_tx, unicast_tx), unicast_rx)
    }
}
