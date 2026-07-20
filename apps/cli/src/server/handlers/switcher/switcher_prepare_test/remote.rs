use super::{
    RepoInfo, build_state, resolve_requested_repo_name, seed_duplicate_remote, select_target_repo,
    write_repo_metadata,
};
use deve_core::models::PeerId;

#[test]
fn select_target_repo_prefers_collision_safe_remote_selector_for_uuid() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected = select_target_repo(&state, false, Some(second_id), None, None, Some(&peer_id))?
        .expect("selector for second wiki repo");
    assert_eq!(selected, second_selector);
    Ok(())
}

#[test]
fn select_target_repo_uses_repo_id_over_host_local_alias() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, first_id, _second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected = select_target_repo(
        &state,
        false,
        Some(first_id),
        Some(&second_selector),
        None,
        Some(&peer_id),
    )?
    .expect("exact RepoId must resolve independently of the host-local alias");
    assert_eq!(selected, first_id.to_string());
    Ok(())
}

#[test]
fn select_target_repo_rejects_remote_uuid_string_without_repo_id() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, _second_selector) = seed_duplicate_remote(&state)?;

    let err = select_target_repo(
        &state,
        false,
        None,
        Some(&second_id.to_string()),
        None,
        Some(&peer_id),
    )
    .expect_err("remote uuid string without repo_id must fail closed");
    assert!(
        err.to_string().contains("Repository UUID not resolved"),
        "remote uuid string without repo_id must fail closed"
    );
    Ok(())
}

#[test]
fn resolve_requested_repo_name_rejects_remote_uuid_string_without_repo_id() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, _second_selector) = seed_duplicate_remote(&state)?;

    let err = resolve_requested_repo_name(&state, Some(&peer_id), &second_id.to_string(), None)
        .expect_err("remote uuid string without repo_id must fail closed");
    assert!(
        err.to_string().contains("Repository UUID not resolved")
            || err.to_string().contains("Remote repository selector")
    );
    Ok(())
}

#[test]
fn resolve_requested_repo_name_accepts_exact_remote_selector_with_uuid() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected =
        resolve_requested_repo_name(&state, Some(&peer_id), &second_selector, Some(second_id))?
            .expect("exact remote selector");
    assert_eq!(selected, second_selector);
    Ok(())
}

#[test]
fn resolve_requested_repo_name_prefers_exact_remote_selector_over_stale_uuid() -> anyhow::Result<()>
{
    let (_dir, state) = build_state()?;
    let (peer_id, first_id, _second_id, second_selector) = seed_duplicate_remote(&state)?;

    let err = resolve_requested_repo_name(&state, Some(&peer_id), &second_selector, Some(first_id))
        .expect_err("stale uuid must not override exact selector");
    assert!(err.to_string().contains("Session repo mismatch:"));
    Ok(())
}

#[test]
fn resolve_requested_repo_name_uses_repo_id_for_duplicate_display_alias() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected = resolve_requested_repo_name(&state, Some(&peer_id), "wiki", Some(second_id))?
        .expect("RepoId must disambiguate duplicate display aliases");
    assert_eq!(selected, second_selector);
    Ok(())
}

#[test]
fn resolve_requested_repo_name_rejects_uuid_shaped_remote_display_name_with_stale_uuid()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-remote");
    let display_uuid = uuid::Uuid::new_v4();
    let stale_uuid = uuid::Uuid::new_v4();
    let peer_dir = state.repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir)?;
    for (uuid, name) in [
        (display_uuid, display_uuid.to_string()),
        (stale_uuid, "shadow-notes".into()),
    ] {
        let db = redb::Database::create(peer_dir.join(format!("{uuid}.redb")))?;
        write_repo_metadata(
            &db,
            &RepoInfo {
                uuid,
                name,
                url: None,
            },
        )?;
    }

    let err = resolve_requested_repo_name(
        &state,
        Some(&peer_id),
        &display_uuid.to_string(),
        Some(stale_uuid),
    )
    .expect_err("uuid-shaped display name must not be overridden by stale uuid");
    assert!(
        err.to_string().contains("Session repo mismatch:")
            || err.to_string().contains("Repository UUID not resolved")
    );
    Ok(())
}

#[test]
fn select_target_repo_does_not_auto_bind_ambiguous_remote_url_matches() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-remote");
    let first = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:shared".into()),
    };
    let second = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:shared".into()),
    };
    state.repo.ensure_shadow_repo_info(&peer_id, &first)?;
    state.repo.ensure_shadow_repo_info(&peer_id, &second)?;

    let err = select_target_repo(
        &state,
        false,
        None,
        None,
        Some("urn:test:shared".into()),
        Some(&peer_id),
    )
    .expect_err("ambiguous remote URL must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous remote repository selector for URL")
    );
    Ok(())
}

#[test]
fn select_target_repo_uses_repo_id_over_current_repo_url() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-remote");
    let first = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    let second = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:wiki-b".into()),
    };
    state.repo.ensure_shadow_repo_info(&peer_id, &first)?;
    state.repo.ensure_shadow_repo_info(&peer_id, &second)?;

    let selected = select_target_repo(
        &state,
        true,
        Some(second.uuid),
        Some("stale-notes"),
        first.url.clone(),
        Some(&peer_id),
    )?
    .expect("exact RepoId match");
    assert_eq!(selected, second.uuid.to_string());
    Ok(())
}

#[test]
fn select_target_repo_uses_repo_id_for_duplicate_current_remote_display_alias() -> anyhow::Result<()>
{
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected = select_target_repo(
        &state,
        true,
        Some(second_id),
        Some("wiki"),
        None,
        Some(&peer_id),
    )?
    .expect("RepoId must disambiguate duplicate remote display aliases");
    assert_eq!(selected, second_selector);
    Ok(())
}
