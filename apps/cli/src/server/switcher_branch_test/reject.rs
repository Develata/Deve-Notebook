//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{browser_session, build_state, unicast_channel};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_unknown_shadow_peer() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(10);

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some("missing-peer".into()),
        Some(11),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(11));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_local_repo_selector() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(12);

    handle_switch_branch(&state, &ch, &mut session, Some("default".into()), Some(13)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(13));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_peer_with_only_broken_shadow_repos() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let bad_peer = PeerId::new("peer-bad");
    let bad_dir = state.repo.remotes_dir().join(bad_peer.to_filename());
    std::fs::create_dir_all(&bad_dir)?;
    std::fs::write(bad_dir.join("broken.redb"), b"not-a-redb")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(22);

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(bad_peer.to_string()),
        Some(23),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(switch_nonce, Some(23));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    Ok(())
}
