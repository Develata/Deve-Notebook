use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) fn require_browser_switch_nonce(
    ch: &DualChannel,
    session: &WsSession,
    switch_nonce: Option<u64>,
    action: &str,
) -> bool {
    if !session.is_browser_session() || switch_nonce.is_some() {
        return true;
    }
    ch.send_protocol_error_with_scope_and_switch_nonce(
        ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!("{action} switch nonce missing"),
        ),
        Some(session.scope_nonce()),
        None,
    );
    false
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn non_browser_switch_nonce_may_be_missing() {
        let (broadcast_tx, _) = broadcast::channel(4);
        let (uni_tx, _uni_rx) = mpsc::channel(4);
        let ch = DualChannel::new(broadcast_tx, uni_tx);
        let session = WsSession::new();

        assert!(require_browser_switch_nonce(
            &ch,
            &session,
            None,
            "repo switch"
        ));
    }
}
