//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_accepts_shadow_peer_even_if_local_repo_stem_matches() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default"))?;
    let local = RepoManager::init(&ledger_dir, 10, Some("peer-remote"), Some("urn:local:peer"))?;
    let local_info = local.get_repo_info()?.expect("local peer repo info");
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
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
