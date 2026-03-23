use super::RepoScopedSyncEngine;
use crate::config::SyncMode;
use crate::ledger::RepoManager;
use crate::models::PeerId;
use std::fs;
use std::sync::Arc;

#[test]
fn strict_engine_load_fails_closed_when_lock_is_poisoned() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = engine.engines.write().expect("write lock");
        panic!("poison repo scoped sync engine");
    }));

    let err = match engine.get_or_create_strict(repo_id) {
        Ok(_) => panic!("strict engine load must fail closed after lock poison"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("lock poisoned"));
    Ok(())
}

#[test]
fn get_returns_none_when_lock_is_poisoned() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = engine.engines.write().expect("write lock");
        panic!("poison repo scoped sync engine");
    }));

    assert!(engine.get(repo_id).is_none());
    let err = engine
        .loaded_repos()
        .expect_err("loaded repos must fail closed after lock poison");
    assert!(err.to_string().contains("poisoned"));
    Ok(())
}

#[test]
fn clone_fails_closed_when_lock_is_poisoned() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = engine.engines.write().expect("write lock");
        panic!("poison repo scoped sync engine");
    }));

    let cloned = engine.clone();
    let err = cloned
        .loaded_repos()
        .expect_err("cloned registry must fail closed after poison");
    assert!(err.to_string().contains("poisoned"));
    Ok(())
}

#[test]
fn clear_fails_closed_when_lock_is_poisoned() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = engine.engines.write().expect("write lock");
        panic!("poison repo scoped sync engine");
    }));

    let err = engine
        .clear()
        .expect_err("clear must fail closed after lock poison");
    assert!(err.to_string().contains("poisoned"));
    Ok(())
}

#[test]
fn strict_engine_load_fails_closed_when_repo_key_is_corrupt() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let key_dir = repo.local_repo_notegit_keys_root("notes")?;
    fs::create_dir_all(&key_dir)?;
    fs::write(key_dir.join("repo.key"), [1, 2, 3, 4])?;

    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);

    let err = match engine.get_or_create_strict(repo_id) {
        Ok(_) => panic!("strict engine load must fail closed on corrupt repo key"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("Failed to load repo key"));
    assert!(engine.get(repo_id).is_none());
    Ok(())
}
