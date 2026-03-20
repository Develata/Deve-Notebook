use super::repo_scope::{resolve_local_counterpart_repo, resolve_session_repo_and_sync};
use super::repo_scope_recovery_support::{build_state, seed_remote_shadow};
use super::session::WsSession;
use deve_core::ledger::{REPO_METADATA, RepoManager};
use deve_core::models::PeerId;

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
        let write = db.begin_write()?;
        write.open_table(REPO_METADATA)?.insert(
            &0,
            bincode::serialize(&deve_core::ledger::RepoInfo {
                uuid,
                name,
                url: None,
            })?
            .as_slice(),
        )?;
        write.commit()?;
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

#[test]
fn resolve_local_counterpart_repo_prefers_repo_uuid_for_remote_scope() -> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;

    let local = resolve_local_counterpart_repo(
        &state,
        &super::repo_scope::ResolvedRepo {
            repo_id: remote_repo_id,
            repo_name: "shadow-notes".into(),
            branch: Some(peer_id),
        },
    )?
    .expect("local counterpart");

    assert!(local.branch.is_none());
    assert_eq!(local.repo_name, "test");
    assert_eq!(local.repo_id, remote_repo_id);
    Ok(())
}

#[test]
fn resolve_local_counterpart_repo_requires_uuid_or_url_match() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let remote_repo_id = uuid::Uuid::new_v4();
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "test")?;

    let local = resolve_local_counterpart_repo(
        &state,
        &super::repo_scope::ResolvedRepo {
            repo_id: remote_repo_id,
            repo_name: "test".into(),
            branch: Some(peer_id),
        },
    )?;

    assert!(local.is_none());
    Ok(())
}

#[test]
fn resolve_local_counterpart_repo_uses_unique_local_url_after_catalog_repair() -> anyhow::Result<()>
{
    let (dir, state, _default_id, _test_id) = build_state()?;
    let _dup = RepoManager::init(dir.path(), 10, Some("mirror"), Some("urn:test"))?;
    let peer_id = PeerId::new("peer-a");
    let remote_repo_id = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: remote_repo_id,
            name: "shadow-test".into(),
            url: Some("urn:test".into()),
        },
    )?;
    let expected_local = state
        .repo
        .find_local_repo_name_by_url("urn:test")?
        .expect("repair must leave a unique local URL owner");
    let expected_info = state
        .repo
        .get_repo_info_for(None, Some(&expected_local))?
        .expect("local repo info");
    let local = resolve_local_counterpart_repo(
        &state,
        &super::repo_scope::ResolvedRepo {
            repo_id: remote_repo_id,
            repo_name: "shadow-test".into(),
            branch: Some(peer_id),
        },
    )?;

    let local = local.expect("local counterpart should recover via unique URL");
    assert!(local.branch.is_none());
    assert_eq!(local.repo_name, expected_local);
    assert_eq!(local.repo_id, expected_info.uuid);
    Ok(())
}

#[test]
fn find_local_repo_name_by_url_fails_closed_when_candidate_metadata_is_unreadable()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let db = state.repo.open_database(None, "default")?.db;
    let txn = db.begin_write()?;
    txn.open_table(REPO_METADATA)?
        .insert(&0, [0_u8, 1, 2, 3].as_slice())?;
    txn.commit()?;

    let err = state
        .repo
        .find_local_repo_name_by_url("urn:default")
        .expect_err("broken local repo metadata must fail closed");
    assert!(
        err.to_string().contains("decode")
            || err.to_string().contains("deserialize")
            || err.to_string().contains("unexpected end")
    );
    Ok(())
}
