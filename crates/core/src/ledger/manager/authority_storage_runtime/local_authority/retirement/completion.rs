//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Terminal owner-lock release and exact Retired publication.

use super::super::{
    HostPathIdentity, LocalAuthorityError, RepoAuthorityCleanupGuard, RepoAuthorityLock,
    RepoAuthoritySlot, RepoId,
};
use std::{
    collections::HashMap,
    sync::{Arc, MutexGuard},
};

#[derive(Clone)]
struct RetirementReservation {
    generation: u64,
    expected_lock_identity: HostPathIdentity,
    removed_database_identity: HostPathIdentity,
}

impl RetirementReservation {
    fn retiring_slot(&self, authority_lock: Option<Arc<RepoAuthorityLock>>) -> RepoAuthoritySlot {
        RepoAuthoritySlot::Retiring {
            generation: self.generation,
            expected_lock_identity: self.expected_lock_identity.clone(),
            removed_database_identity: self.removed_database_identity.clone(),
            authority_lock,
        }
    }

    fn retired_slot(&self) -> RepoAuthoritySlot {
        RepoAuthoritySlot::Retired {
            prior_generation: self.generation,
            expected_lock_identity: self.expected_lock_identity.clone(),
            removed_database_identity: self.removed_database_identity.clone(),
        }
    }

    fn matches_unowned_retiring(&self, slot: Option<&RepoAuthoritySlot>) -> bool {
        matches!(
            slot,
            Some(RepoAuthoritySlot::Retiring {
                generation,
                expected_lock_identity,
                removed_database_identity,
                authority_lock: None,
            }) if *generation == self.generation
                && expected_lock_identity == &self.expected_lock_identity
                && removed_database_identity == &self.removed_database_identity
        )
    }
}

impl RepoAuthorityCleanupGuard {
    /// Called only by the composed `RepoManager` after it has verified both
    /// the owner-issued DB checkpoint and exact catalog tombstone retirement.
    pub(crate) fn complete_inner(&mut self) -> Result<(), LocalAuthorityError> {
        self.complete_inner_with_hooks(|| {}, || {}, false)
    }

    fn complete_inner_with_hooks(
        &mut self,
        before_unlock: impl FnOnce(),
        after_unlock: impl FnOnce(),
        inject_release_failure: bool,
    ) -> Result<(), LocalAuthorityError> {
        if self.settled {
            return Ok(());
        }
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        let Some(slot) = slots.remove(&self.repo_id) else {
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} committed-cleanup slot disappeared",
                self.repo_id
            )));
        };
        let RepoAuthoritySlot::CommittedCleanup {
            generation,
            authority_lock,
            expected_lock_identity,
            removed_database_identity,
            db_path,
            cleanup_capability_issued,
        } = slot
        else {
            slots.insert(self.repo_id, slot);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} is not committed-cleanup authority",
                self.repo_id
            )));
        };
        if generation != self.generation || db_path != self.db_path || !cleanup_capability_issued {
            slots.insert(
                self.repo_id,
                RepoAuthoritySlot::CommittedCleanup {
                    generation,
                    authority_lock,
                    expected_lock_identity,
                    removed_database_identity,
                    db_path,
                    cleanup_capability_issued,
                },
            );
            return Err(LocalAuthorityError::StaleGeneration {
                repo_id: self.repo_id,
                expected: self.generation,
                actual: generation,
            });
        }
        if Arc::strong_count(&authority_lock) != 1 {
            slots.insert(
                self.repo_id,
                RepoAuthoritySlot::CommittedCleanup {
                    generation,
                    authority_lock,
                    expected_lock_identity,
                    removed_database_identity,
                    db_path,
                    cleanup_capability_issued,
                },
            );
            return Err(LocalAuthorityError::Busy(self.repo_id));
        }
        let authority_lock = match Arc::try_unwrap(authority_lock) {
            Ok(authority_lock) => authority_lock,
            Err(authority_lock) => {
                slots.insert(
                    self.repo_id,
                    RepoAuthoritySlot::CommittedCleanup {
                        generation,
                        authority_lock,
                        expected_lock_identity,
                        removed_database_identity,
                        db_path,
                        cleanup_capability_issued,
                    },
                );
                return Err(LocalAuthorityError::Busy(self.repo_id));
            }
        };
        let reservation = RetirementReservation {
            generation,
            expected_lock_identity,
            removed_database_identity,
        };
        slots.insert(self.repo_id, reservation.retiring_slot(None));
        drop(slots);
        before_unlock();
        let slots = match self.inner.slots.lock() {
            Ok(slots) => slots,
            Err(_) => {
                std::mem::forget(authority_lock);
                return Err(LocalAuthorityError::Poisoned);
            }
        };
        if !reservation.matches_unowned_retiring(slots.get(&self.repo_id)) {
            let displaced = drop_and_replace_with_retiring_lock(
                slots,
                self.repo_id,
                &reservation,
                Arc::new(authority_lock),
            );
            drop(displaced);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} lost its exact pre-release reservation",
                self.repo_id
            )));
        }
        drop(slots);
        let release = if inject_release_failure {
            Err((
                authority_lock,
                std::io::Error::other("injected local authority unlock failure"),
            ))
        } else {
            authority_lock.release()
        };
        if let Err((authority_lock, error)) = release {
            let authority_lock = Arc::new(authority_lock);
            let mut slots = match self.inner.slots.lock() {
                Ok(slots) => slots,
                Err(_) => {
                    std::mem::forget(authority_lock);
                    return Err(LocalAuthorityError::Poisoned);
                }
            };
            if reservation.matches_unowned_retiring(slots.get(&self.repo_id)) {
                let Some(RepoAuthoritySlot::Retiring {
                    authority_lock: retained,
                    ..
                }) = slots.get_mut(&self.repo_id)
                else {
                    unreachable!("exact retiring reservation was just matched");
                };
                if retained.is_none() {
                    *retained = Some(authority_lock);
                    return Err(LocalAuthorityError::Io(error));
                }
            }
            let displaced = slots.insert(
                self.repo_id,
                reservation.retiring_slot(Some(authority_lock)),
            );
            drop(slots);
            drop(displaced);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} lost its exact failed-release reservation",
                self.repo_id
            )));
        }
        after_unlock();
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        if !reservation.matches_unowned_retiring(slots.get(&self.repo_id)) {
            let displaced = slots.insert(self.repo_id, reservation.retiring_slot(None));
            drop(slots);
            drop(displaced);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} lost its exact post-release reservation",
                self.repo_id
            )));
        }
        let displaced = slots.insert(self.repo_id, reservation.retired_slot());
        drop(slots);
        drop(displaced);
        self.settled = true;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn complete_with_hooks_for_test(
        &mut self,
        before_unlock: impl FnOnce(),
        after_unlock: impl FnOnce(),
        inject_release_failure: bool,
    ) -> Result<(), LocalAuthorityError> {
        self.complete_inner_with_hooks(before_unlock, after_unlock, inject_release_failure)
    }
}

fn drop_and_replace_with_retiring_lock(
    mut slots: MutexGuard<'_, HashMap<RepoId, RepoAuthoritySlot>>,
    repo_id: RepoId,
    reservation: &RetirementReservation,
    authority_lock: Arc<RepoAuthorityLock>,
) -> Option<RepoAuthoritySlot> {
    let displaced = slots.insert(repo_id, reservation.retiring_slot(Some(authority_lock)));
    drop(slots);
    displaced
}

pub(super) fn slot_generation(slot: Option<&RepoAuthoritySlot>) -> Option<u64> {
    match slot? {
        RepoAuthoritySlot::Opening { .. } => None,
        RepoAuthoritySlot::Reopening { generation, .. }
        | RepoAuthoritySlot::Preparing { generation, .. }
        | RepoAuthoritySlot::ReopeningPrepared { generation, .. }
        | RepoAuthoritySlot::ReopeningRepairRequired { generation, .. }
        | RepoAuthoritySlot::RepairRequired { generation }
        | RepoAuthoritySlot::Active { generation, .. }
        | RepoAuthoritySlot::Quiescing { generation, .. }
        | RepoAuthoritySlot::CommittedCleanup { generation, .. }
        | RepoAuthoritySlot::Retiring { generation, .. } => Some(*generation),
        RepoAuthoritySlot::Retired {
            prior_generation, ..
        } => Some(*prior_generation),
    }
}
