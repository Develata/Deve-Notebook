use super::{RepoInfo, build_state, resolve_requested_repo_name, write_repo_metadata};
use deve_core::models::PeerId;

#[test]
fn resolve_requested_repo_name_fails_closed_when_remote_display_name_is_ambiguous_even_with_uuid()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-remote");
    let peer_dir = state.repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir)?;
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
    for (stem, info) in [("wiki-a", &first), ("wiki-b", &second)] {
        let db = redb::Database::create(peer_dir.join(format!("{stem}.redb")))?;
        write_repo_metadata(&db, info)?;
    }

    let err = resolve_requested_repo_name(&state, Some(&peer_id), "wiki", Some(second.uuid))
        .expect_err("ambiguous remote display name must not fall back to repo id");
    assert!(
        err.to_string()
            .contains("ambiguous remote repository selector")
    );
    Ok(())
}

#[test]
fn select_target_repo_fails_closed_when_remote_url_recovery_sees_repo_without_url()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-remote");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "notes".into(),
            url: Some("urn:test:notes".into()),
        },
    )?;
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "broken".into(),
            url: None,
        },
    )?;

    let err = super::select_target_repo(
        &state,
        false,
        None,
        None,
        Some("urn:test:notes".into()),
        Some(&peer_id),
    )
    .expect_err("remote URL recovery must fail closed on missing candidate URL");
    assert!(err.to_string().contains("repository URL not resolved"));
    Ok(())
}
