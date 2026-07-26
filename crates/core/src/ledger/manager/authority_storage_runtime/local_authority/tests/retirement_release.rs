//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Lock-release linearization regressions for terminal local authority cleanup.

use super::*;
use std::sync::mpsc;

struct WorkerRelease(mpsc::Sender<()>);

impl Drop for WorkerRelease {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

#[test]
fn retiring_reservation_blocks_admission_while_owner_lock_releases() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    let mut cleanup = runtime
        .quiesce_for_test(repo_id, 1, Duration::from_secs(1))?
        .into_committed_cleanup()?;
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_guard = WorkerRelease(release_tx);

    std::thread::scope(|scope| -> anyhow::Result<()> {
        let completion = scope.spawn(move || {
            cleanup.complete_with_hooks_for_test(
                || {
                    entered_tx
                        .send(())
                        .expect("retirement completion hook must signal entry");
                    release_rx
                        .recv_timeout(Duration::from_secs(5))
                        .expect("retirement completion hook release timed out");
                },
                || {},
                false,
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(5))?;
        let slots = match runtime.inner.slots.try_lock() {
            Ok(slots) => slots,
            Err(_) => panic!("retirement hook must not retain the authority map mutex"),
        };
        assert!(matches!(
            slots.get(&repo_id),
            Some(RepoAuthoritySlot::Retiring { generation: 1, .. })
        ));
        drop(slots);
        assert!(matches!(
            runtime.lease(repo_id),
            Err(LocalAuthorityError::Quiescing(id)) if id == repo_id
        ));
        assert!(matches!(
            LocalAuthorityRuntime::open_existing(dir.path(), repo_id),
            Err(LocalAuthorityError::Busy(id)) if id == repo_id
        ));
        drop(release_guard);
        completion.join().expect("retirement completion thread")?;
        Ok(())
    })?;

    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::Retired {
            prior_generation: 1
        })
    );
    Ok(())
}

#[test]
fn unlock_failure_retains_fail_closed_retiring_capability() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    let mut cleanup = runtime
        .quiesce_for_test(repo_id, 1, Duration::from_secs(1))?
        .into_committed_cleanup()?;

    assert!(matches!(
        cleanup.complete_with_hooks_for_test(|| {}, || {}, true),
        Err(LocalAuthorityError::Io(_))
    ));
    assert_eq!(
        runtime.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::Retiring { generation: 1 })
    );
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::Quiescing(id)) if id == repo_id
    ));
    assert!(matches!(
        LocalAuthorityRuntime::open_existing(dir.path(), repo_id),
        Err(LocalAuthorityError::Busy(id)) if id == repo_id
    ));
    Ok(())
}

#[derive(Clone, Copy)]
enum ReservationMutation {
    Remove,
    SwapIdentities,
}

const RESERVATION_MUTATIONS: [ReservationMutation; 2] = [
    ReservationMutation::Remove,
    ReservationMutation::SwapIdentities,
];

#[test]
fn pre_release_reservation_drift_retains_owner_lock_repair_debt() -> anyhow::Result<()> {
    for mutation in RESERVATION_MUTATIONS {
        let (dir, runtime, repo_id) = new_runtime()?;
        let mut cleanup = runtime
            .quiesce_for_test(repo_id, 1, Duration::from_secs(1))?
            .into_committed_cleanup()?;
        let (expected_lock_identity, removed_database_identity) =
            committed_cleanup_identities(&runtime, repo_id)?;
        let inner = runtime.inner.clone();

        assert!(matches!(
            cleanup.complete_with_hooks_for_test(
                move || mutate_retiring_reservation(&inner, repo_id, mutation),
                || {},
                false,
            ),
            Err(LocalAuthorityError::Invariant(_))
        ));
        assert_retiring_debt(
            &runtime,
            repo_id,
            &expected_lock_identity,
            &removed_database_identity,
            true,
        )?;
        assert!(matches!(
            LocalAuthorityRuntime::open_existing(dir.path(), repo_id),
            Err(LocalAuthorityError::Busy(id)) if id == repo_id
        ));
    }
    Ok(())
}

#[test]
fn post_release_reservation_drift_stays_retiring_repair_debt() -> anyhow::Result<()> {
    for mutation in RESERVATION_MUTATIONS {
        let (_dir, runtime, repo_id) = new_runtime()?;
        let mut cleanup = runtime
            .quiesce_for_test(repo_id, 1, Duration::from_secs(1))?
            .into_committed_cleanup()?;
        let (expected_lock_identity, removed_database_identity) =
            committed_cleanup_identities(&runtime, repo_id)?;
        let inner = runtime.inner.clone();

        assert!(matches!(
            cleanup.complete_with_hooks_for_test(
                || {},
                move || mutate_retiring_reservation(&inner, repo_id, mutation),
                false,
            ),
            Err(LocalAuthorityError::Invariant(_))
        ));
        assert_retiring_debt(
            &runtime,
            repo_id,
            &expected_lock_identity,
            &removed_database_identity,
            false,
        )?;
    }
    Ok(())
}

fn committed_cleanup_identities(
    runtime: &LocalAuthorityRuntime,
    repo_id: RepoId,
) -> anyhow::Result<(HostPathIdentity, HostPathIdentity)> {
    let slots = runtime
        .inner
        .slots
        .lock()
        .map_err(|_| LocalAuthorityError::Poisoned)?;
    let Some(RepoAuthoritySlot::CommittedCleanup {
        expected_lock_identity,
        removed_database_identity,
        ..
    }) = slots.get(&repo_id)
    else {
        panic!("cleanup must own the committed-cleanup reservation");
    };
    Ok((
        expected_lock_identity.clone(),
        removed_database_identity.clone(),
    ))
}

fn mutate_retiring_reservation(
    inner: &LocalAuthorityInner,
    repo_id: RepoId,
    mutation: ReservationMutation,
) {
    let mut slots = inner
        .slots
        .lock()
        .unwrap_or_else(|_| panic!("authority map mutex poisoned"));
    match mutation {
        ReservationMutation::Remove => {
            slots.remove(&repo_id);
        }
        ReservationMutation::SwapIdentities => {
            let Some(RepoAuthoritySlot::Retiring {
                expected_lock_identity,
                removed_database_identity,
                ..
            }) = slots.get_mut(&repo_id)
            else {
                panic!("release hook requires retiring reservation");
            };
            std::mem::swap(expected_lock_identity, removed_database_identity);
        }
    }
}

fn assert_retiring_debt(
    runtime: &LocalAuthorityRuntime,
    repo_id: RepoId,
    expected_lock_identity: &HostPathIdentity,
    removed_database_identity: &HostPathIdentity,
    lock_retained: bool,
) -> anyhow::Result<()> {
    let slots = runtime
        .inner
        .slots
        .lock()
        .map_err(|_| LocalAuthorityError::Poisoned)?;
    let Some(RepoAuthoritySlot::Retiring {
        generation,
        expected_lock_identity: actual_lock_identity,
        removed_database_identity: actual_database_identity,
        authority_lock,
    }) = slots.get(&repo_id)
    else {
        panic!("reservation drift must leave retiring repair debt");
    };
    assert_eq!(*generation, 1);
    assert_eq!(actual_lock_identity, expected_lock_identity);
    assert_eq!(actual_database_identity, removed_database_identity);
    assert_eq!(authority_lock.is_some(), lock_retained);
    drop(slots);
    assert!(matches!(
        runtime.lease(repo_id),
        Err(LocalAuthorityError::Quiescing(id)) if id == repo_id
    ));
    Ok(())
}
