//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract

use super::*;
use crate::ledger::init::init_core_tables;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use std::time::{Duration, Instant};

mod admission_error;
mod reopening;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoAuthoritySlotSnapshot {
    Opening,
    Reopening { generation: u64 },
    Preparing { generation: u64 },
    ReopeningPrepared { generation: u64 },
    RepairRequired { generation: u64 },
    Active { generation: u64 },
    Quiescing { generation: u64 },
    CommittedCleanup { generation: u64 },
    Retired { prior_generation: u64 },
}

fn new_runtime() -> anyhow::Result<(tempfile::TempDir, LocalAuthorityRuntime, RepoId)> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("local"))?;
    let repo_id = Uuid::new_v4();
    let (runtime, prepared) =
        LocalAuthorityRuntime::prepare_new_initialized(dir.path(), repo_id, |db| {
            init_core_tables(db)?;
            RepoManager::initialize_repo_info_in_new_db(
                db,
                &RepoInfo {
                    uuid: repo_id,
                    name: repo_id.to_string(),
                    url: Some(format!("urn:uuid:{repo_id}")),
                },
            )?;
            crate::ledger::source_control::init_tables(db)?;
            Ok(())
        })?;
    prepared.activate_for_test(&runtime)?;
    Ok((dir, runtime, repo_id))
}

#[test]
fn lease_is_repo_and_generation_exact() -> anyhow::Result<()> {
    let (_dir, runtime, repo_id) = new_runtime()?;
    let lease = runtime.lease(repo_id)?;
    let expected_stem = repo_id.to_string();
    assert_eq!(lease.repo_id(), repo_id);
    assert_eq!(lease.generation(), 1);
    assert_eq!(
        lease.db_path().file_stem().and_then(|v| v.to_str()),
        Some(expected_stem.as_str())
    );
    Ok(())
}

#[test]
fn removal_snapshot_requires_the_only_live_authority_lease() -> anyhow::Result<()> {
    let (_dir, runtime, repo_id) = new_runtime()?;
    let writer = runtime.lease(repo_id)?;
    let removal = runtime.lease(repo_id)?;
    assert!(matches!(
        removal.removal_snapshot(),
        Err(LocalAuthorityError::Busy(id)) if id == repo_id
    ));
    drop(writer);
    assert_eq!(removal.removal_snapshot()?.repo_id(), repo_id);
    Ok(())
}

#[test]
fn quiescing_rejects_new_leases_and_timeout_restores_active() -> anyhow::Result<()> {
    let (_dir, runtime, repo_id) = new_runtime()?;
    let lease = runtime.lease(repo_id)?;
    let error = match runtime.quiesce_for_test(repo_id, 1, Duration::from_millis(5)) {
        Ok(_) => panic!("held lease must time out quiesce"),
        Err(error) => error,
    };
    assert!(matches!(error, LocalAuthorityError::DrainTimeout(id) if id == repo_id));
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::Active { generation: 1 })
    );
    drop(lease);
    assert_eq!(runtime.lease(repo_id)?.generation(), 1);
    Ok(())
}

#[test]
fn explicit_cleanup_retires_without_ordinary_reopen() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    let snapshot = runtime.lease(repo_id)?.removal_snapshot()?;
    let quiesce = runtime.quiesce_for_test(repo_id, 1, Duration::from_secs(1))?;
    assert_eq!(quiesce.repo_id(), repo_id);
    assert_eq!(quiesce.generation(), 1);
    let cleanup = quiesce.into_committed_cleanup()?;
    assert_eq!(cleanup.repo_id(), repo_id);
    assert_eq!(cleanup.generation(), 1);
    assert_eq!(
        cleanup.db_path(),
        dir.path().join("local").join(format!("{repo_id}.redb"))
    );
    let error = match LocalAuthorityRuntime::open_existing(dir.path(), repo_id) {
        Ok(_) => panic!("cleanup guard must retain the cross-process owner lock"),
        Err(error) => error,
    };
    assert!(matches!(error, LocalAuthorityError::Busy(id) if id == repo_id));
    let mut cleanup = cleanup;
    let checkpoint =
        cleanup.advance_database_cleanup(&snapshot, &snapshot.initial_database_checkpoint())?;
    let checkpoint = cleanup.advance_database_cleanup(&snapshot, &checkpoint)?;
    cleanup.verify_database_cleanup_complete(&snapshot, &checkpoint)?;
    cleanup.complete_inner()?;
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::Retired {
            prior_generation: 1
        })
    );
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::Retired(id)) if id == repo_id
    ));
    let reopened_lock = crate::utils::fs::open_regular_file_lock_existing(
        snapshot.authority_lock().path(),
        "retired local authority lock",
    )?;
    reopened_lock.try_lock()?;
    crate::utils::fs::ensure_open_file_matches_identity(
        &reopened_lock,
        snapshot.authority_lock(),
        "retired local authority lock",
    )?;
    reopened_lock.unlock()?;
    Ok(())
}

#[test]
fn dropped_committed_cleanup_retains_slot_and_process_lock() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    let cleanup = runtime
        .quiesce_for_test(repo_id, 1, Duration::from_secs(1))?
        .into_committed_cleanup()?;
    drop(cleanup);
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::CommittedCleanup { generation: 1 })
    );
    assert!(matches!(
        LocalAuthorityRuntime::open_existing(dir.path(), repo_id),
        Err(LocalAuthorityError::Busy(id)) if id == repo_id
    ));
    Ok(())
}

#[test]
fn dropped_quiesce_guard_restores_active_without_generation_change() -> anyhow::Result<()> {
    let (_dir, runtime, repo_id) = new_runtime()?;
    let guard = runtime.quiesce_for_test(repo_id, 1, Duration::from_secs(1))?;
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::Quiescing(id)) if id == repo_id
    ));
    guard.rollback()?;
    assert_eq!(runtime.lease(repo_id)?.generation(), 1);
    Ok(())
}

#[test]
fn committed_cut_close_race_stays_fail_closed_until_process_recovery() -> anyhow::Result<()> {
    let (_dir, runtime, repo_id) = new_runtime()?;
    let guard = runtime.quiesce_for_test(repo_id, 1, Duration::from_secs(1))?;
    let leaked_resource = guard
        .resources
        .as_ref()
        .expect("quiesce guard owns resources")
        .clone();
    let error = match guard.into_committed_cleanup() {
        Ok(_) => panic!("a late resource reference must fail the committed cut"),
        Err(error) => error,
    };
    assert!(matches!(error, LocalAuthorityError::Busy(id) if id == repo_id));
    drop(leaked_resource);
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::Quiescing { generation: 1 })
    );
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::Quiescing(id)) if id == repo_id
    ));
    Ok(())
}

#[test]
fn second_runtime_cannot_open_same_repo_while_owner_lives() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    let error = match LocalAuthorityRuntime::open_existing(dir.path(), repo_id) {
        Ok(_) => panic!("second owner must fail closed"),
        Err(error) => error,
    };
    assert!(matches!(error, LocalAuthorityError::Busy(id) if id == repo_id));
    drop(runtime);
    LocalAuthorityRuntime::open_existing(dir.path(), repo_id)?;
    Ok(())
}

#[test]
fn bootstrap_discovery_uses_and_releases_the_same_owner_lock() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    drop(runtime);

    let discovery = LocalAuthorityDiscovery::new(dir.path());
    let lease = discovery.lease(repo_id)?;
    assert_eq!(lease.repo_id(), repo_id);
    let error = match LocalAuthorityRuntime::open_existing(dir.path(), repo_id) {
        Ok(_) => panic!("discovery owner must exclude a second runtime"),
        Err(error) => error,
    };
    assert!(matches!(error, LocalAuthorityError::Busy(id) if id == repo_id));
    drop(lease);
    drop(discovery);

    LocalAuthorityRuntime::open_existing(dir.path(), repo_id)?;
    Ok(())
}

#[test]
fn panicking_existing_admission_releases_opening_reservation() -> anyhow::Result<()> {
    let (dir, runtime, _primary_repo_id) = new_runtime()?;
    let repo_id = Uuid::new_v4();
    let (secondary, prepared) =
        LocalAuthorityRuntime::prepare_new_initialized(dir.path(), repo_id, |db| {
            init_core_tables(db)?;
            RepoManager::initialize_repo_info_in_new_db(
                db,
                &RepoInfo {
                    uuid: repo_id,
                    name: repo_id.to_string(),
                    url: Some(format!("urn:uuid:{repo_id}")),
                },
            )?;
            Ok(())
        })?;
    prepared.activate_for_test(&secondary)?;
    drop(secondary);

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = runtime.admit_existing_with_hook_for_test(repo_id, || {
            panic!("injected existing-admission panic")
        });
    }));
    assert!(panic.is_err());
    assert_eq!(runtime.snapshot_for_test(repo_id)?, None);
    assert_eq!(runtime.admit_existing(repo_id)?.repo_id(), repo_id);
    Ok(())
}

#[test]
fn secondary_create_is_owned_by_the_existing_runtime() -> anyhow::Result<()> {
    let (dir, runtime, _primary_repo_id) = new_runtime()?;
    let repo_id = Uuid::new_v4();
    let prepared = runtime.create_repo_initialized(repo_id, |db| {
        init_core_tables(db)?;
        RepoManager::initialize_repo_info_in_new_db(
            db,
            &RepoInfo {
                uuid: repo_id,
                name: repo_id.to_string(),
                url: Some(format!("urn:uuid:{repo_id}")),
            },
        )?;
        crate::ledger::source_control::init_tables(db)?;
        Ok(())
    })?;
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::Preparing { generation: 1 })
    );
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::Busy(id)) if id == repo_id
    ));
    drop(prepared);
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 1 })
    );
    assert!(matches!(
        runtime.create_repo_initialized(repo_id, |_| Ok(())),
        Err(LocalAuthorityError::Busy(id)) if id == repo_id
    ));
    assert!(
        dir.path()
            .join("local")
            .join(format!("{repo_id}.redb"))
            .is_file()
    );
    Ok(())
}

#[test]
fn initial_create_remains_preparing_until_explicit_activation() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("local"))?;
    let repo_id = Uuid::new_v4();
    let (runtime, prepared) =
        LocalAuthorityRuntime::prepare_new_initialized(dir.path(), repo_id, |db| {
            init_core_tables(db)?;
            RepoManager::initialize_repo_info_in_new_db(
                db,
                &RepoInfo {
                    uuid: repo_id,
                    name: repo_id.to_string(),
                    url: Some(format!("urn:uuid:{repo_id}")),
                },
            )?;
            Ok(())
        })?;

    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::Preparing { generation: 1 })
    );
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::Busy(id)) if id == repo_id
    ));
    drop(prepared);
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 1 })
    );
    Ok(())
}

#[test]
fn failed_secondary_initialization_never_publishes_active_authority() -> anyhow::Result<()> {
    let (dir, runtime, _primary_repo_id) = new_runtime()?;
    let repo_id = Uuid::new_v4();

    let error = match runtime.create_repo_initialized(repo_id, |_| {
        Err(LocalAuthorityError::Invariant(
            "injected initialization failure".to_string(),
        ))
    }) {
        Ok(_) => panic!("failed initialization must not return a lease"),
        Err(error) => error,
    };
    assert!(matches!(error, LocalAuthorityError::Invariant(_)));
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 1 })
    );
    assert!(
        dir.path()
            .join("local")
            .join(format!("{repo_id}.redb"))
            .is_file(),
        "an unverified path must be left for explicit repair, never guessed safe to delete"
    );
    Ok(())
}

#[test]
fn fresh_create_lock_only_residual_is_explicit_repair_debt() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("local"))?;
    std::fs::create_dir_all(
        crate::utils::notegit::host_dir(dir.path()).join("repo-authority-locks"),
    )?;
    let repo_id = Uuid::new_v4();
    drop(crate::utils::fs::create_regular_file_lock_new(
        &super::resource::authority_lock_path(dir.path(), repo_id),
        "interrupted authority lock",
    )?);
    let runtime = LocalAuthorityRuntime::empty(dir.path());

    assert!(
        runtime
            .create_repo_initialized(repo_id, |_| Ok(()))
            .is_err()
    );
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 1 })
    );
    assert!(!super::resource::database_path(dir.path(), repo_id).exists());
    Ok(())
}

#[test]
fn existing_inspection_does_not_create_missing_host_directories() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("local"))?;
    let runtime = LocalAuthorityRuntime::empty(dir.path());
    let repo_id = Uuid::new_v4();

    assert!(
        runtime
            .inspect_existing_stem(&repo_id.to_string(), |_| Ok(()))
            .is_err()
    );
    assert!(!crate::utils::notegit::host_dir(dir.path()).exists());
    Ok(())
}

#[test]
fn panicking_secondary_initialization_is_sealed_for_repair() -> anyhow::Result<()> {
    let (_dir, runtime, _primary_repo_id) = new_runtime()?;
    let repo_id = Uuid::new_v4();

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = runtime.create_repo_initialized(repo_id, |_| -> Result<(), LocalAuthorityError> {
            panic!("injected initializer panic")
        });
    }));
    assert!(panic.is_err());
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 1 })
    );
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::RepairRequired(id)) if id == repo_id
    ));
    Ok(())
}

#[test]
fn panicking_repair_inspection_restores_repair_slot() -> anyhow::Result<()> {
    let (_dir, runtime, _primary_repo_id) = new_runtime()?;
    let repo_id = Uuid::new_v4();
    let prepared = runtime.create_repo_initialized(repo_id, |db| {
        init_core_tables(db)?;
        RepoManager::initialize_repo_info_in_new_db(
            db,
            &RepoInfo {
                uuid: repo_id,
                name: repo_id.to_string(),
                url: Some(format!("urn:uuid:{repo_id}")),
            },
        )?;
        crate::ledger::source_control::init_tables(db)?;
        Ok(())
    })?;
    drop(prepared);

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = runtime.inspect_existing_stem(&repo_id.to_string(), |_| -> anyhow::Result<()> {
            panic!("injected inspection panic")
        });
    }));
    assert!(panic.is_err());
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 1 })
    );
    let observed = runtime.inspect_existing_stem(&repo_id.to_string(), |db| {
        Ok(RepoManager::read_local_repo_info_from_db(db)?.expect("repair inspection metadata"))
    })?;
    assert_eq!(observed.uuid, repo_id);
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 1 })
    );
    Ok(())
}

#[test]
fn drain_wakes_when_the_exact_lease_is_released() -> anyhow::Result<()> {
    let (_dir, runtime, repo_id) = new_runtime()?;
    let lease = runtime.lease(repo_id)?;
    let started = Instant::now();
    std::thread::scope(|scope| -> anyhow::Result<()> {
        scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            drop(lease);
        });
        let guard = runtime.quiesce_for_test(repo_id, 1, Duration::from_secs(2))?;
        assert!(started.elapsed() < Duration::from_secs(1));
        guard.rollback()?;
        Ok(())
    })?;
    Ok(())
}
