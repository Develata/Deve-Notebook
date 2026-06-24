//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::{build_state, seed_shadow_doc};
use crate::server::{
    channel::DualChannel, handlers::source_control::handle_get_doc_diff, session::WsSession,
};
use deve_core::models::PeerId;
use deve_core::protocol::{ScPathTarget, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_diff_is_allowed_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let doc_id = seed_shadow_doc(state.repo.as_ref(), &peer_id, test_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "req-1".into(),
        ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
        domain: None,
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::DocDiff {
            repo_id,
            doc_id: actual_doc_id,
            new_content,
            ..
        }) => {
            assert_eq!(repo_id, Some(test_id));
            assert_eq!(actual_doc_id, Some(doc_id));
            assert_eq!(new_content, "remote");
        }
        other => panic!("expected DocDiff, got {:?}", other),
    }
    Ok(())
}
