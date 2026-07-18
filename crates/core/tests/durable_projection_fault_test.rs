use deve_core::ledger::RepoManager;
use deve_core::sync::SyncManager;
use std::sync::Arc;

#[test]
fn durable_projection_fault_survives_sync_manager_restart() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let ledger_dir = tmp.path().join("ledger");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(tmp.path().join("notes"))?;
    let repo_stem = repo.local_repo_name().to_string();
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo metadata").uuid;
    let sync = SyncManager::new_checked(repo.clone())?;

    assert!(!sync.is_projection_degraded(&repo_stem));
    sync.mark_projection_writeback_fault(&repo_stem);
    assert!(sync.is_projection_degraded(&repo_stem));

    let restarted = SyncManager::new_checked(repo.clone())?;
    assert!(restarted.is_projection_degraded(&repo_stem));

    let journal = std::fs::read_to_string(ledger_dir.join(".host").join("projection-faults.toml"))?;
    assert!(journal.contains("repo_id"));
    assert!(journal.contains(&repo_id.to_string()));
    assert!(journal.contains("projection_writeback_failed"));
    Ok(())
}

#[test]
fn rebuild_projection_clears_durable_projection_fault() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let ledger_dir = tmp.path().join("ledger");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(tmp.path().join("notes"))?;
    let repo_stem = repo.local_repo_name().to_string();
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(repo.clone())?;

    sync.mark_projection_writeback_fault(&repo_stem);
    assert!(SyncManager::new_checked(repo.clone())?.is_projection_degraded(&repo_stem));

    sync.rebuild_projection_local_repo(&repo_stem)?;
    assert!(!SyncManager::new_checked(repo.clone())?.is_projection_degraded(&repo_stem));
    assert!(
        !ledger_dir
            .join(".host")
            .join("projection-faults.toml")
            .try_exists()?
    );
    Ok(())
}
