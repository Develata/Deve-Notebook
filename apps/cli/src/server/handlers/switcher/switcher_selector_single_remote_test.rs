use super::switcher_prepare_test::build_state;
use super::switcher_selector::select_target_repo;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;

fn seed_single_remote_target(
    state: &std::sync::Arc<crate::server::AppState>,
) -> anyhow::Result<(PeerId, uuid::Uuid, uuid::Uuid)> {
    let peer_id = PeerId::new("peer-remote");
    let remote_id = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: remote_id,
            name: "default".into(),
            url: Some("urn:remote:default".into()),
        },
    )?;
    let dir = state.repo.ledger_dir().to_path_buf();
    RepoManager::init(&dir, 10, Some("test"), Some("urn:test"))?;
    let test_id = state
        .repo
        .get_repo_info_for(None, Some("test"))?
        .expect("test repo info")
        .uuid;
    Ok((peer_id, test_id, remote_id))
}

#[test]
fn select_target_repo_falls_back_to_only_remote_repo_when_url_hint_misses() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, test_id, remote_id) = seed_single_remote_target(&state)?;

    let selected = select_target_repo(
        &state,
        true,
        Some(test_id),
        Some("test"),
        Some("urn:test".into()),
        Some(&peer_id),
    )?
    .expect("single remote selector fallback");
    assert_eq!(selected, remote_id.to_string());
    Ok(())
}

#[test]
fn select_target_repo_falls_back_to_only_remote_repo_when_name_hint_misses() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, test_id, remote_id) = seed_single_remote_target(&state)?;

    let selected = select_target_repo(
        &state,
        true,
        Some(test_id),
        Some("test"),
        None,
        Some(&peer_id),
    )?
    .expect("single remote selector fallback");
    assert_eq!(selected, remote_id.to_string());
    Ok(())
}
