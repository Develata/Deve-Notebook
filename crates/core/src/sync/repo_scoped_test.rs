use super::RepoScopedSyncEngine;
use crate::config::SyncMode;
use crate::ledger::RepoManager;
use crate::models::PeerId;
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
