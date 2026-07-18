//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_keeps_exact_remote_selector_with_repo_uuid() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("default"),
        Some("urn:default"),
    )?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let peer_id = PeerId::new("peer-remote");
    let first = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    let second = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-b".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &first)?;
    repo.ensure_shadow_repo_info(&peer_id, &second)?;
    let selector = repo
        .find_remote_repo_selector_by_id(&peer_id, second.uuid)?
        .expect("collision-safe selector");
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    let (ch, _uni_rx) = unicast_channel(&state);
    let mut session = browser_session(0);
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(selector, Some(second.uuid));

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(1),
    )
    .await;

    assert_eq!(session.active_branch, Some(peer_id));
    assert_eq!(session.active_repo.as_deref(), Some("wiki"));
    assert_eq!(session.active_repo_id, Some(second.uuid));
    Ok(())
}
