//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_repo;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_disambiguates_duplicate_remote_display_name_by_repo_id()
-> anyhow::Result<()> {
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
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(4);
    session.switch_branch(Some(peer_id.to_string()));

    handle_switch_repo(
        &state,
        &ch,
        &mut session,
        "wiki".into(),
        Some(second.uuid),
        Some(5),
    )
    .await;

    assert_eq!(session.active_repo.as_deref(), Some("wiki"));
    assert_eq!(session.active_repo_id, Some(second.uuid));
    match uni_rx.recv().await {
        Some(ServerMessage::RepoSwitched {
            branch,
            name,
            uuid,
            switch_nonce,
        }) => {
            assert_eq!(branch.as_deref(), Some(peer_id.as_str()));
            assert_eq!(name, "wiki");
            assert_eq!(uuid, second.uuid.to_string());
            assert_eq!(switch_nonce, Some(5));
        }
        other => panic!("expected RepoSwitched, got {other:?}"),
    }
    Ok(())
}
