use super::switcher_prepare_test::build_state;
use super::switcher_selector::select_target_repo;
use deve_core::ledger::RepoInfo;
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
    let ledger_dir = state.repo.ledger_dir().to_path_buf();
    let projection_base = ledger_dir.parent().expect("ledger parent").join("notes");
    let test_id = crate::server::catalog_repo_support::catalog_additional_repo(
        state.repo.as_ref(),
        &ledger_dir,
        "test",
        &projection_base,
        10,
        Some("urn:test"),
    )?;
    Ok((peer_id, test_id, remote_id))
}

#[test]
fn select_target_repo_rejects_only_remote_repo_when_exact_id_is_missing() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, test_id, remote_id) = seed_single_remote_target(&state)?;
    assert_ne!(test_id, remote_id);

    let error = select_target_repo(
        &state,
        true,
        Some(test_id),
        Some("test"),
        Some("urn:test".into()),
        Some(&peer_id),
    )
    .expect_err("the only remote repo must not replace a missing exact RepoId");
    assert!(error.to_string().contains(&test_id.to_string()));
    Ok(())
}

#[test]
fn select_target_repo_rejects_alias_fallback_when_exact_id_is_missing() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, test_id, remote_id) = seed_single_remote_target(&state)?;
    assert_ne!(test_id, remote_id);

    let error = select_target_repo(
        &state,
        true,
        Some(test_id),
        Some("test"),
        None,
        Some(&peer_id),
    )
    .expect_err("a host-local alias must not replace a missing exact RepoId");
    assert!(error.to_string().contains(&test_id.to_string()));
    Ok(())
}
