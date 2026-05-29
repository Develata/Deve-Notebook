//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::{build_state, seed_shadow_doc_with_url};
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_get_doc_diff, session::WsSession,
};
use deve_core::models::PeerId;
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_diff_fails_closed_when_no_local_counterpart_repo_exists() -> anyhow::Result<()> {
    let (_dir, state, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-remote-only");
    let remote_repo_id = uuid::Uuid::new_v4();
    let doc_id = seed_shadow_doc_with_url(
        state.repo.as_ref(),
        &peer_id,
        remote_repo_id,
        "shadow-remote-only",
        "urn:remote-only",
    )?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-remote-only".into(), Some(remote_repo_id));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "req-no-local".into(),
        ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::StorageNotFound);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("No local repository matched"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}
