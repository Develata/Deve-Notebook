use super::{
    REPO_METADATA, RepoInfo, build_state, resolve_requested_repo_name, seed_duplicate_remote,
    select_target_repo,
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
fn select_target_repo_prefers_exact_remote_selector_over_stale_uuid() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, first_id, _second_id, second_selector) = seed_duplicate_remote(&state)?;

    let err = select_target_repo(
        &state,
        false,
        Some(first_id),
        Some(&second_selector),
        None,
        Some(&peer_id),
    )
    .expect_err("stale uuid must not override exact selector");
    assert!(err.to_string().contains("Session repo mismatch:"));
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
fn resolve_requested_repo_name_accepts_exact_remote_selector_without_uuid() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, _second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected = resolve_requested_repo_name(&state, Some(&peer_id), &second_selector, None)?
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
fn resolve_requested_repo_name_fails_closed_when_exact_selector_conflicts_with_repo_id()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, second_selector) = seed_duplicate_remote(&state)?;

    let err = resolve_requested_repo_name(&state, Some(&peer_id), "wiki", Some(second_id))
        .expect_err("exact remote selector must beat stale repo id");
    assert!(err.to_string().contains("Session repo mismatch:"));
    assert!(err.to_string().contains(&second_selector));
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
    for (stem, uuid, name) in [
        ("shadow-display", display_uuid, display_uuid.to_string()),
        ("shadow-notes", stale_uuid, "shadow-notes".into()),
    ] {
        let db = redb::Database::create(peer_dir.join(format!("{stem}.redb")))?;
        let write = db.begin_write()?;
        write.open_table(REPO_METADATA)?.insert(
            &0,
            bincode::serialize(&RepoInfo {
                uuid,
                name,
                url: None,
            })?
            .as_slice(),
        )?;
        write.commit()?;
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
fn select_target_repo_prefers_current_repo_url_over_stale_uuid() -> anyhow::Result<()> {
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
    .expect("canonical URL match");
    assert_eq!(selected, "wiki");
    Ok(())
}

#[test]
fn select_target_repo_fails_closed_when_exact_current_remote_name_conflicts_with_repo_id()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, _second_selector) = seed_duplicate_remote(&state)?;

    let err = select_target_repo(
        &state,
        true,
        Some(second_id),
        Some("wiki"),
        None,
        Some(&peer_id),
    )
    .expect_err("exact current remote selector must beat stale repo id");
    assert!(err.to_string().contains("Session repo mismatch:"));
    Ok(())
}
