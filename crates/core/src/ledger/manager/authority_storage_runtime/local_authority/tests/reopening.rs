//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Same-process Retired -> ReopeningPrepared owner proofs.

use super::*;

fn retire(
    runtime: &LocalAuthorityRuntime,
    repo_id: RepoId,
) -> anyhow::Result<RepoAuthorityRemovalSnapshot> {
    let snapshot = runtime.lease(repo_id)?.removal_snapshot()?;
    let mut cleanup = runtime
        .quiesce_for_test(repo_id, 1, Duration::from_secs(1))?
        .into_committed_cleanup()?;
    let checkpoint =
        cleanup.advance_database_cleanup(&snapshot, &snapshot.initial_database_checkpoint())?;
    let checkpoint = cleanup.advance_database_cleanup(&snapshot, &checkpoint)?;
    cleanup.verify_database_cleanup_complete(&snapshot, &checkpoint)?;
    cleanup.complete_inner()?;
    Ok(snapshot)
}

#[test]
fn retired_authority_prepares_a_fresh_owner_bound_generation() -> anyhow::Result<()> {
    let (_dir, runtime, repo_id) = new_runtime()?;
    let snapshot = retire(&runtime, repo_id)?;

    let old_database_identity = snapshot.database().object_identity();
    let prepared = runtime.prepare_retired_repo_initialized(repo_id, |db| {
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
        Some(RepoAuthoritySlotSnapshot::ReopeningPrepared { generation: 2 })
    );
    assert!(snapshot.authority_lock().revalidate()?);
    assert_ne!(
        HostPathIdentity::capture(
            prepared.resources.db_path.as_path(),
            HostPathKind::RegularFile
        )?
        .object_identity(),
        old_database_identity
    );
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::Busy(id)) if id == repo_id
    ));

    prepared.activate_for_test(&runtime)?;
    assert_eq!(runtime.lease(repo_id)?.generation(), 2);
    Ok(())
}

#[test]
fn retired_authority_never_recreates_a_missing_or_replaced_lock() -> anyhow::Result<()> {
    for replace in [false, true] {
        let (_dir, runtime, repo_id) = new_runtime()?;
        let snapshot = retire(&runtime, repo_id)?;

        std::fs::remove_file(snapshot.authority_lock().path())?;
        if replace {
            drop(crate::utils::fs::create_regular_file_new(
                snapshot.authority_lock().path(),
                "replacement authority lock",
            )?);
        }
        let error = match runtime.prepare_retired_repo_initialized(repo_id, |_| Ok(())) {
            Ok(_) => panic!("missing or replaced retired lock must fail closed"),
            Err(error) => error,
        };
        assert!(matches!(error, LocalAuthorityError::RepairRequired(id) if id == repo_id));
        assert_eq!(
            runtime.snapshot_for_test(repo_id)?,
            Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 2 })
        );
        if !replace {
            assert!(!snapshot.authority_lock().path().exists());
        }
    }
    Ok(())
}

#[test]
fn reopening_failure_and_prepared_drop_keep_the_exact_lock_owned() -> anyhow::Result<()> {
    for fail_during_initialize in [true, false] {
        let (dir, runtime, repo_id) = new_runtime()?;
        retire(&runtime, repo_id)?;
        let result = runtime.prepare_retired_repo_initialized(repo_id, |db| {
            init_core_tables(db)?;
            RepoManager::initialize_repo_info_in_new_db(
                db,
                &RepoInfo {
                    uuid: repo_id,
                    name: repo_id.to_string(),
                    url: Some(format!("urn:uuid:{repo_id}")),
                },
            )?;
            if fail_during_initialize {
                return Err(LocalAuthorityError::Invariant(
                    "injected reopening initialization failure".to_string(),
                ));
            }
            crate::ledger::source_control::init_tables(db)?;
            Ok(())
        });
        if fail_during_initialize {
            assert!(result.is_err());
        } else {
            drop(result?);
        }
        assert_eq!(
            runtime.snapshot_for_test(repo_id)?,
            Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 2 })
        );
        assert!(matches!(
            LocalAuthorityRuntime::open_existing(dir.path(), repo_id),
            Err(LocalAuthorityError::Busy(id)) if id == repo_id
        ));
    }
    Ok(())
}

#[test]
fn reopening_rejects_replaced_database_parent_without_publishing() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    retire(&runtime, repo_id)?;
    let local = dir.path().join("local");
    let displaced = dir.path().join("local-displaced");
    std::fs::rename(&local, &displaced)?;
    std::fs::create_dir(&local)?;

    let error = match runtime.prepare_retired_repo_initialized(repo_id, |_| Ok(())) {
        Ok(_) => panic!("replaced canonical DB parent must fail closed"),
        Err(error) => error,
    };
    assert!(matches!(error, LocalAuthorityError::RepairRequired(id) if id == repo_id));
    assert!(!local.join(format!("{repo_id}.redb")).exists());
    Ok(())
}

#[test]
fn cold_runtime_cannot_reconstruct_retired_proof_from_paths() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    retire(&runtime, repo_id)?;
    drop(runtime);
    let cold = LocalAuthorityRuntime::empty(dir.path());
    assert!(matches!(
        cold.prepare_retired_repo_initialized(repo_id, |_| Ok(())),
        Err(LocalAuthorityError::NotAdmitted(id)) if id == repo_id
    ));
    Ok(())
}
