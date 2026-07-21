//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_accepts_shadow_peer_even_if_local_repo_stem_matches() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger_dir,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let local_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &repo,
        &ledger_dir,
        "peer-remote",
        &projection_base,
        10,
        Some("urn:local:peer"),
    )?;
    let local_info = repo
        .get_local_repo_info_by_id(local_id)?
        .expect("local peer repo info");
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(&peer_id, &local_info)?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(1);

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(2),
    )
    .await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::BranchSwitched { success: true, .. })
    ));
    assert_eq!(session.active_branch, Some(peer_id));
    Ok(())
}
