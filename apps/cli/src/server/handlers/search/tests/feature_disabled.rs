use super::super::handle_search;
use crate::server::channel::DualChannel;
use crate::server::edit_state_test_support::edit_harness;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test]
async fn handler_returns_structured_error_when_search_feature_is_not_compiled() -> anyhow::Result<()>
{
    let h = edit_harness(false)?;
    let (tx, mut rx) = mpsc::channel(8);
    let ch = DualChannel::new(h.state.tx.clone(), tx);
    let mut session = WsSession::new();

    handle_search(
        &h.state,
        &ch,
        &mut session,
        "search-1".into(),
        "needle".into(),
        10,
        Some(44),
    )
    .await;

    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(error.detail.as_deref(), Some("Search feature not enabled"));
            assert_eq!(scope_nonce, Some(44));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}
