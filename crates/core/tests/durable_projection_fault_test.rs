use deve_core::sync::SyncManager;
use std::sync::Arc;

mod common;

#[test]
fn durable_projection_fault_survives_sync_manager_restart() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let ledger_dir = tmp.path().join("ledger");
    let (repo, _repo_id) = common::init_cataloged_repo(&ledger_dir, &tmp.path().join("notes"))?;
    let repo_stem = repo.local_repo_name().to_string();
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(repo.clone())?;

    assert!(!sync.is_projection_degraded(&repo_stem));
    sync.mark_projection_writeback_fault(&repo_stem)?;
    assert!(sync.is_projection_degraded(&repo_stem));

    let restarted = SyncManager::new_checked(repo.clone())?;
    assert!(restarted.is_projection_degraded(&repo_stem));

    assert!(
        !ledger_dir
            .join(".host")
            .join("projection-faults.toml")
            .try_exists()?
    );
    Ok(())
}

#[test]
fn rebuild_projection_clears_durable_projection_fault() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let ledger_dir = tmp.path().join("ledger");
    let (repo, _repo_id) = common::init_cataloged_repo(&ledger_dir, &tmp.path().join("notes"))?;
    let repo_stem = repo.local_repo_name().to_string();
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(repo.clone())?;

    sync.mark_projection_writeback_fault(&repo_stem)?;
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
