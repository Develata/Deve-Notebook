//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::handlers::switcher::handle_switch_repo;
use super::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_exact_fails_closed_when_remote_repo_id_is_stale() -> anyhow::Result<()> {
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
    let info = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &info)?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(30);
    session.switch_branch(Some(peer_id.to_string()));

    handle_switch_repo(
        &state,
        &ch,
        &mut session,
        "wiki".into(),
        Some(uuid::Uuid::new_v4()),
        Some(31),
    )
    .await;

    assert!(matches!(
        uni_rx.recv().await,
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce: Some(31),
            ..
        }) if error.code == ServerErrorCode::ScRepoContextInvalid
            && error.detail.as_deref().is_some_and(|detail| {
                detail.contains("Repository UUID not resolved for repository selector wiki")
            })
    ));
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    Ok(())
}
