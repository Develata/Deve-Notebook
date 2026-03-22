use super::switcher_prepare::prepare_repo_switch;
use super::switcher_selector::{resolve_requested_repo_name, select_target_repo};
use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

mod remote;
mod remote_fail_closed;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
        tx,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            identity_key.peer_id(),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_service: None,
        identity_key,
    });
    Ok((dir, state))
}

pub(super) fn seed_duplicate_remote(
    state: &Arc<AppState>,
) -> anyhow::Result<(PeerId, uuid::Uuid, uuid::Uuid, String)> {
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
    state.repo.ensure_shadow_repo_info(&peer_id, &first)?;
    state.repo.ensure_shadow_repo_info(&peer_id, &second)?;
    let second_selector = state
        .repo
        .find_remote_repo_selector_by_id(&peer_id, second.uuid)?
        .expect("selector for second repo");
    Ok((peer_id, first.uuid, second.uuid, second_selector))
}

#[test]
fn select_target_repo_rejects_local_uuid_string_without_repo_id() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    let test_id = state
        .repo
        .get_repo_info_for(None, Some("test"))?
        .expect("test repo info")
        .uuid;

    let err = select_target_repo(&state, false, None, Some(&test_id.to_string()), None, None)
        .expect_err("local uuid string without repo_id must fail closed");
    assert!(
        err.to_string().contains("Local repo not found for name")
            || err
                .to_string()
                .contains("Local repository selector not resolved")
    );
    Ok(())
}

#[test]
fn select_target_repo_prefers_exact_local_stem_over_stale_uuid() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    let default_id = state
        .repo
        .get_repo_info_for(None, Some("default"))?
        .expect("default repo info")
        .uuid;

    let err = select_target_repo(&state, false, Some(default_id), Some("test"), None, None)
        .expect_err("stale uuid must not override exact local stem");
    assert!(err.to_string().contains("Session repo mismatch:"));
    Ok(())
}

#[test]
fn resolve_requested_repo_name_fails_closed_on_stale_local_alias_after_metadata_drift()
-> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let wiki = RepoManager::init(dir.path(), 10, Some("wiki"), Some("urn:wiki"))?;
    let wiki_info = wiki.get_repo_info()?.expect("wiki info");
    let wiki_db = state.repo.open_database(None, "wiki")?.db;
    let txn = wiki_db.begin_write()?;
    txn.open_table(REPO_METADATA)?.insert(
        &0,
        bincode::serialize(&RepoInfo {
            uuid: wiki_info.uuid,
            name: "legacy-wiki".into(),
            url: wiki_info.url.clone(),
        })?
        .as_slice(),
    )?;
    txn.commit()?;

    let err = resolve_requested_repo_name(&state, None, "legacy-wiki", None)
        .expect_err("stale local alias must fail closed");
    assert!(
        err.to_string()
            .contains("metadata name drifted to legacy-wiki")
    );
    Ok(())
}

#[test]
fn select_target_repo_fails_closed_on_stale_local_alias_after_metadata_drift() -> anyhow::Result<()>
{
    let (dir, state) = build_state()?;
    let wiki = RepoManager::init(dir.path(), 10, Some("wiki"), Some("urn:wiki"))?;
    let wiki_info = wiki.get_repo_info()?.expect("wiki info");
    let wiki_db = state.repo.open_database(None, "wiki")?.db;
    let txn = wiki_db.begin_write()?;
    txn.open_table(REPO_METADATA)?.insert(
        &0,
        bincode::serialize(&RepoInfo {
            uuid: wiki_info.uuid,
            name: "legacy-wiki".into(),
            url: wiki_info.url.clone(),
        })?
        .as_slice(),
    )?;
    txn.commit()?;

    let err = select_target_repo(&state, false, None, Some("legacy-wiki"), None, None)
        .expect_err("stale local alias must fail closed");
    assert!(
        err.to_string()
            .contains("metadata name drifted to legacy-wiki")
    );
    Ok(())
}

#[test]
fn resolve_requested_repo_name_prefers_exact_local_stem_over_stale_uuid() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    let default_id = state
        .repo
        .get_repo_info_for(None, Some("default"))?
        .expect("default repo info")
        .uuid;

    let err = resolve_requested_repo_name(&state, None, "test", Some(default_id))
        .expect_err("stale uuid must not override exact local stem");
    assert!(err.to_string().contains("Session repo mismatch:"));
    Ok(())
}

#[test]
fn select_target_repo_fails_closed_on_ambiguous_local_alias() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    RepoManager::init(dir.path(), 10, Some("notes-a"), Some("urn:notes-a"))?;
    RepoManager::init(dir.path(), 10, Some("notes-b"), Some("urn:notes-b"))?;
    for repo_name in ["notes-a", "notes-b"] {
        let repo_uuid = state
            .repo
            .get_repo_info_for(None, Some(repo_name))?
            .expect("repo info")
            .uuid;
        let db = state.repo.open_database(None, repo_name)?.db;
        let txn = db.begin_write()?;
        txn.open_table(REPO_METADATA)?.insert(
            &0,
            bincode::serialize(&RepoInfo {
                uuid: repo_uuid,
                name: "wiki".into(),
                url: Some(format!("urn:{repo_name}")),
            })?
            .as_slice(),
        )?;
        txn.commit()?;
    }

    let err = select_target_repo(&state, false, None, Some("wiki"), None, None)
        .expect_err("stale local alias must fail closed");
    assert!(err.to_string().contains("metadata name drifted to wiki"));
    Ok(())
}

#[test]
fn select_target_repo_fails_closed_when_local_url_candidate_is_unreadable() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let db = state.repo.open_database(None, "default")?.db;
    let txn = db.begin_write()?;
    txn.open_table(REPO_METADATA)?
        .insert(&0, [0_u8, 1, 2, 3].as_slice())?;
    txn.commit()?;

    let err = select_target_repo(&state, false, None, None, Some("urn:default".into()), None)
        .expect_err("broken local repo metadata must fail closed during URL recovery");
    assert!(
        err.to_string().contains("decode")
            || err.to_string().contains("deserialize")
            || err.to_string().contains("unexpected end")
    );
    Ok(())
}

#[test]
fn prepare_repo_switch_rejects_local_repo_without_uuid_metadata() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    let db = state.repo.open_database(None, "test")?.db;
    let txn = db.begin_write()?;
    txn.open_table(REPO_METADATA)?.remove(&0)?;
    txn.commit()?;

    let err = match prepare_repo_switch(&state, None, "test".into()) {
        Ok(_) => anyhow::bail!("local switch must fail without repo uuid metadata"),
        Err(err) => err,
    };
    assert!(
        err.to_string().contains("repository metadata missing")
            || err
                .to_string()
                .contains("Local repository UUID not resolved for selector: test")
    );
    Ok(())
}
