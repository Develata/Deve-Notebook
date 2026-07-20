use super::switcher_prepare::prepare_repo_switch;
use super::switcher_selector::{resolve_requested_repo_name, select_target_repo};
use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::codec;
use deve_core::config::SyncMode;
use deve_core::ledger::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
    RepoInfo, RepoManager,
};
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

mod remote;
mod remote_fail_closed;

pub(super) fn write_repo_metadata(db: &redb::Database, info: &RepoInfo) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        let bytes = codec::encode(info)?;
        table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
    }
    txn.commit()?;
    Ok(())
}

fn write_invalid_repo_metadata(db: &redb::Database) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        table.insert(&REPO_INFO_METADATA_KEY, [0_u8, 1, 2, 3].as_slice())?;
    }
    txn.commit()?;
    Ok(())
}

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo_with_url(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
        Some("urn:default".to_string()),
    )?;
    let repo = Arc::new(cataloged.repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
        tx,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            identity_key.peer_id(),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_available: false,
        identity_key,
    });
    Ok((dir, state))
}

fn init_local_repo(dir: &TempDir, url: &str) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    let cataloged = crate::test_support::init_cataloged_repo_with_url(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
        Some(url.to_string()),
    )?;
    Ok((cataloged.repo, cataloged.repo_id))
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
    let (_test_repo, test_id) = init_local_repo(&dir, "urn:test")?;

    let err = select_target_repo(&state, false, None, Some(&test_id.to_string()), None, None)
        .expect_err("local uuid string without repo_id must fail closed");
    assert!(err.to_string().contains("Repository UUID not resolved"));
    Ok(())
}

#[test]
fn select_target_repo_uses_local_repo_id_over_other_repo_selector() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let (_test_repo, test_id) = init_local_repo(&dir, "urn:test")?;
    let default_id = state.repo.get_repo_info()?.expect("default repo info").uuid;

    let selected = select_target_repo(
        &state,
        false,
        Some(default_id),
        Some(&test_id.to_string()),
        None,
        None,
    )?
    .expect("exact local RepoId must override a conflicting selector hint");
    assert_eq!(selected, default_id.to_string());
    Ok(())
}

#[test]
fn resolve_requested_repo_name_fails_closed_on_stale_local_alias_after_metadata_drift()
-> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let (wiki, wiki_id) = init_local_repo(&dir, "urn:wiki")?;
    let wiki_info = wiki.get_repo_info()?.expect("wiki info");
    let wiki_db = state.repo.open_database(None, &wiki_id.to_string())?.db;
    write_repo_metadata(
        &wiki_db,
        &RepoInfo {
            uuid: wiki_info.uuid,
            name: "legacy-wiki".into(),
            url: wiki_info.url.clone(),
        },
    )?;

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
    let (wiki, wiki_id) = init_local_repo(&dir, "urn:wiki")?;
    let wiki_info = wiki.get_repo_info()?.expect("wiki info");
    let wiki_db = state.repo.open_database(None, &wiki_id.to_string())?.db;
    write_repo_metadata(
        &wiki_db,
        &RepoInfo {
            uuid: wiki_info.uuid,
            name: "legacy-wiki".into(),
            url: wiki_info.url.clone(),
        },
    )?;

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
    let (_test_repo, test_id) = init_local_repo(&dir, "urn:test")?;
    let default_id = state.repo.get_repo_info()?.expect("default repo info").uuid;

    let err = resolve_requested_repo_name(&state, None, &test_id.to_string(), Some(default_id))
        .expect_err("stale uuid must not override exact local stem");
    assert!(err.to_string().contains("Session repo mismatch:"));
    Ok(())
}

#[test]
fn select_target_repo_fails_closed_on_ambiguous_local_alias() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let (_notes_a, notes_a_id) = init_local_repo(&dir, "urn:notes-a")?;
    let (_notes_b, notes_b_id) = init_local_repo(&dir, "urn:notes-b")?;
    let mut targets = Vec::new();
    for (repo_uuid, url) in [(notes_a_id, "urn:notes-a"), (notes_b_id, "urn:notes-b")] {
        let db = state.repo.open_database(None, &repo_uuid.to_string())?.db;
        targets.push((repo_uuid, url, db));
    }
    for (repo_uuid, url, db) in targets {
        write_repo_metadata(
            &db,
            &RepoInfo {
                uuid: repo_uuid,
                name: "wiki".into(),
                url: Some(url.to_string()),
            },
        )?;
    }

    let err = select_target_repo(&state, false, None, Some("wiki"), None, None)
        .expect_err("stale local alias must fail closed");
    assert!(err.to_string().contains("metadata name drifted to wiki"));
    Ok(())
}

#[test]
fn select_target_repo_fails_closed_when_local_url_candidate_is_unreadable() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default repo info").uuid;
    let db = state.repo.open_database(None, &default_id.to_string())?.db;
    write_invalid_repo_metadata(&db)?;

    let err = select_target_repo(&state, false, None, None, Some("urn:default".into()), None)
        .expect_err("broken local repo metadata must fail closed during URL recovery");
    assert!(
        err.to_string().contains("decode")
            || err.to_string().contains("deserialize")
            || err.to_string().contains("deserialization")
            || err.to_string().contains("postcard")
            || err.to_string().contains("unexpected end")
    );
    Ok(())
}

#[tokio::test]
async fn prepare_repo_switch_rejects_local_repo_without_uuid_metadata() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let (_test_repo, test_id) = init_local_repo(&dir, "urn:test")?;
    let db = state.repo.open_database(None, &test_id.to_string())?.db;
    let txn = db.begin_write()?;
    txn.open_table(REPO_METADATA)?
        .remove(&REPO_INFO_METADATA_KEY)?;
    txn.commit()?;

    let err = match prepare_repo_switch(&state, None, test_id.to_string()).await {
        Ok(_) => anyhow::bail!("local switch must fail without repo uuid metadata"),
        Err(err) => err,
    };
    assert!(
        err.to_string().contains("repository metadata missing")
            || err.to_string().contains(&format!(
                "Local repository UUID not resolved for selector: {test_id}"
            ))
    );
    Ok(())
}
