use super::switcher_prepare::{
    prepare_repo_switch, resolve_requested_repo_name, select_target_repo,
};
use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
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

fn seed_duplicate_remote(
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
fn select_target_repo_prefers_collision_safe_remote_selector_for_uuid() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected = select_target_repo(&state, false, Some(second_id), None, None, Some(&peer_id))?
        .expect("selector for second wiki repo");
    assert_eq!(selected, second_selector);
    Ok(())
}

#[test]
fn select_target_repo_recovers_remote_selector_from_uuid_string_without_repo_id()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected = select_target_repo(
        &state,
        false,
        None,
        Some(&second_id.to_string()),
        None,
        Some(&peer_id),
    )?
    .expect("selector for second wiki repo");
    assert_eq!(selected, second_selector);
    Ok(())
}

#[test]
fn resolve_requested_repo_name_recovers_remote_selector_from_uuid_string_without_repo_id()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (peer_id, _first_id, second_id, second_selector) = seed_duplicate_remote(&state)?;

    let selected =
        resolve_requested_repo_name(&state, Some(&peer_id), &second_id.to_string(), None)?
            .expect("selector for second wiki repo");
    assert_eq!(selected, second_selector);
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
fn select_target_repo_recovers_local_stem_from_uuid_string_without_repo_id() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    let test_id = state
        .repo
        .get_repo_info_for(None, Some("test"))?
        .expect("test repo info")
        .uuid;

    let selected = select_target_repo(&state, false, None, Some(&test_id.to_string()), None, None)?
        .expect("canonical local repo stem");
    assert_eq!(selected, "test");
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
fn resolve_requested_repo_name_prefers_canonical_local_stem_after_metadata_drift()
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

    let selected = resolve_requested_repo_name(&state, None, "legacy-wiki", None)?
        .expect("canonical local selector");
    assert_eq!(selected, "wiki");
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
        err.to_string()
            .contains("Local repository UUID not resolved for selector: test")
    );
    Ok(())
}
