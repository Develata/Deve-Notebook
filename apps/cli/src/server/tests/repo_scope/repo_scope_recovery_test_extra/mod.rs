use super::repo_scope::resolve_session_repo_and_sync;
use super::repo_scope_recovery_support::{build_state, seed_remote_shadow};
use super::session::WsSession;
use deve_core::ledger::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use deve_core::models::PeerId;

fn write_repo_metadata(
    db: &redb::Database,
    info: &deve_core::ledger::RepoInfo,
) -> anyhow::Result<()> {
    let write = db.begin_write()?;
    {
        let mut table = write.open_table(REPO_METADATA)?;
        let version = bincode::serialize(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        let bytes = bincode::serialize(info)?;
        table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
    }
    write.commit()?;
    Ok(())
}

#[test]
fn resolve_session_repo_recovers_collision_safe_remote_selector_from_uuid() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    seed_remote_shadow(&state, &peer_id, first, "wiki")?;
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: second,
            name: "wiki".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    )?;

    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.active_repo_id = Some(second);

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;
    let expected_selector = state
        .repo
        .find_remote_repo_selector_by_id(&peer_id, second)?
        .expect("selector for duplicate remote repo");

    assert_eq!(resolved.branch, Some(peer_id));
    assert_eq!(resolved.repo_id, second);
    assert_eq!(resolved.repo_name, expected_selector);
    assert_eq!(
        session.active_repo.as_deref(),
        Some(resolved.repo_name.as_str())
    );
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_stale_exact_remote_selector_uuid_pair() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    seed_remote_shadow(&state, &peer_id, first, "wiki")?;
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: second,
            name: "wiki".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    )?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("wiki".into(), Some(second));

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("stale exact selector must fail closed");
    assert!(err.to_string().contains("stale remote scope:"));
    assert!(err.to_string().contains("Session repo mismatch"));
    Ok(())
}

#[test]
fn resolve_session_repo_accepts_exact_collision_safe_remote_selector_without_uuid()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    seed_remote_shadow(&state, &peer_id, first, "wiki")?;
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: second,
            name: "wiki".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    )?;
    let expected_selector = state
        .repo
        .find_remote_repo_selector_by_id(&peer_id, second)?
        .expect("selector for duplicate remote repo");

    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(expected_selector.clone(), None);

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;
    assert_eq!(resolved.branch, Some(peer_id));
    assert_eq!(resolved.repo_id, second);
    assert_eq!(resolved.repo_name, expected_selector);
    assert_eq!(
        session.active_repo.as_deref(),
        Some(resolved.repo_name.as_str())
    );
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_uuid_shaped_remote_display_name_with_stale_uuid()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let display_uuid = uuid::Uuid::new_v4();
    let stale_uuid = uuid::Uuid::new_v4();
    let peer_dir = state.repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir)?;
    for (stem, uuid, name) in [
        ("shadow-display", display_uuid, display_uuid.to_string()),
        ("shadow-notes", stale_uuid, "shadow-notes".into()),
    ] {
        let db = redb::Database::create(peer_dir.join(format!("{stem}.redb")))?;
        write_repo_metadata(
            &db,
            &deve_core::ledger::RepoInfo {
                uuid,
                name,
                url: None,
            },
        )?;
    }

    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(display_uuid.to_string(), Some(stale_uuid));

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("uuid-shaped display name must not be overridden by stale uuid");
    assert!(
        err.to_string()
            .contains("Remote repository selector not resolved")
            || err.to_string().contains("Session repo mismatch")
    );
    Ok(())
}

mod local_counterpart;
