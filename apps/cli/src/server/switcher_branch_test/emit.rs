//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{browser_session, build_state, unicast_channel};
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_emits_scope_messages_after_success_ack() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let local = state
        .repo
        .get_repo_info()?
        .expect("local repo info must exist");
    let peer_id = PeerId::new("peer-remote");
    state.repo.ensure_shadow_repo_info(&peer_id, &local)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(16);

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(17),
    )
    .await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::BranchSwitched {
            success: true,
            switch_nonce: Some(17),
            ..
        })
    ));
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::RepoList {
            branch: Some(_),
            ..
        })
    ));
    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::RepoSwitched {
            branch: Some(_),
            switch_nonce: Some(17),
            ..
        })
    ));
    Ok(())
}
