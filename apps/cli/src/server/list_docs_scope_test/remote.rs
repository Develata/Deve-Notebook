//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::support::build_state;
use crate::server::{
    channel::DualChannel, handlers::listing::handle_list_docs, session::WsSession,
};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_on_unbound_shadow_branch_returns_repo_unbound() -> anyhow::Result<()> {
    let (_dir, state, _test_repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("missing-shadow".into()));

    handle_list_docs(&state, &ch, &mut session, None, None).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Remote branch not available:"))
            );
        }
        other => panic!("expected stale shadow ProtocolError, got {:?}", other),
    }
    assert!(
        uni_rx.try_recv().is_err(),
        "must not send empty doc/tree payload"
    );
    Ok(())
}
