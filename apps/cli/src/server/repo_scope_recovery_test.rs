use super::repo_scope::{resolve_local_counterpart_repo, resolve_session_repo_and_sync};
use super::{AppState, session::WsSession, tree_state::RepoTreeRegistry};
use crate::server::security;
use deve_core::config::SyncMode;
use deve_core::ledger::{REPO_METADATA, RepoManager};
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    let test_repo = RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(32);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key,
        }),
        default_id,
        test_id,
    ))
}

fn seed_remote_shadow(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    repo_name: &str,
) -> anyhow::Result<()> {
    let info = deve_core::ledger::RepoInfo {
        uuid: repo_id,
        name: repo_name.to_string(),
        url: None,
    };
    state.repo.ensure_shadow_repo_info(peer_id, &info)?;
    Ok(())
}

#[test]
fn resolve_session_repo_recovers_remote_repo_name_from_uuid() -> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.active_repo_id = Some(remote_repo_id);

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;

    assert_eq!(resolved.branch, Some(peer_id));
    assert_eq!(resolved.repo_id, remote_repo_id);
    assert_eq!(resolved.repo_name, "shadow-notes");
    assert_eq!(session.active_repo.as_deref(), Some("shadow-notes"));
    Ok(())
}

#[test]
fn resolve_session_repo_recovers_local_repo_name_from_uuid_string_without_bound_id()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo(test_id.to_string(), None);

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;

    assert!(resolved.branch.is_none());
    assert_eq!(resolved.repo_id, test_id);
    assert_eq!(resolved.repo_name, "test");
    assert_eq!(session.active_repo.as_deref(), Some("test"));
    Ok(())
}

#[test]
fn resolve_session_repo_recovers_remote_scope_from_uuid_when_name_is_stale() -> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("stale-name".into(), Some(remote_repo_id));

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;

    assert_eq!(resolved.branch, Some(peer_id));
    assert_eq!(resolved.repo_id, remote_repo_id);
    assert_eq!(resolved.repo_name, "shadow-notes");
    assert_eq!(session.active_repo.as_deref(), Some("shadow-notes"));
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_remote_selector_without_uuid_when_name_is_unrecoverable()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("stale-name".into(), None);

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("remote stale selector without uuid must fail");
    assert!(
        err.to_string().contains("Active repository not selected")
            || err.to_string().contains("Remote session lost repo name")
    );
    Ok(())
}

#[test]
fn resolve_session_repo_recovers_remote_selector_from_uuid_string_without_bound_id()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(remote_repo_id.to_string(), None);

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;

    assert_eq!(resolved.branch, Some(peer_id));
    assert_eq!(resolved.repo_id, remote_repo_id);
    assert_eq!(resolved.repo_name, "shadow-notes");
    assert_eq!(session.active_repo.as_deref(), Some("shadow-notes"));
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
