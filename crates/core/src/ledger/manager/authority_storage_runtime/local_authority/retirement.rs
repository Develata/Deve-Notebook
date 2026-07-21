//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Pre-commit quiescence and post-commit authority retirement capabilities.

use super::{
    LocalAuthorityError, RepoAuthorityCleanupGuard, RepoAuthorityQuiesceGuard,
    RepoAuthorityResources, RepoAuthoritySlot,
};
use crate::models::RepoId;
use std::path::Path;
use std::sync::Arc;

#[allow(dead_code)] // Consumed by the approved R3/R4 removal coordinator.
impl RepoAuthorityQuiesceGuard {
    pub(crate) fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Re-opens the admission gate without changing its generation.
    pub(crate) fn rollback(mut self) -> Result<(), LocalAuthorityError> {
        self.rollback_inner()
    }

    /// Crosses the irreversible authority cut after the caller has durably
    /// committed the removal tombstone. The database closes here, while the
    /// persistent owner lock moves into the slot itself.
    pub(crate) fn into_committed_cleanup(
        mut self,
    ) -> Result<RepoAuthorityCleanupGuard, LocalAuthorityError> {
        let resources = self.resources.take().ok_or_else(|| {
            LocalAuthorityError::Invariant(format!(
                "RepoId {} quiesce guard has no resources",
                self.repo_id
            ))
        })?;
        let mut slots = match self.inner.slots.lock() {
            Ok(slots) => slots,
            Err(_) => {
                self.resources = Some(resources);
                return Err(LocalAuthorityError::Poisoned);
            }
        };
        let Some(slot) = slots.remove(&self.repo_id) else {
            self.resources = Some(resources);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} quiescing slot disappeared",
                self.repo_id
            )));
        };
        let RepoAuthoritySlot::Quiescing {
            generation,
            resources: Some(slot_resources),
            leases: 0,
        } = slot
        else {
            slots.insert(self.repo_id, slot);
            self.resources = Some(resources);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} is not in the drain-complete quiescing state",
                self.repo_id
            )));
        };
        if generation != self.generation || !Arc::ptr_eq(&slot_resources, &resources) {
            slots.insert(
                self.repo_id,
                RepoAuthoritySlot::Quiescing {
                    generation,
                    resources: Some(slot_resources),
                    leases: 0,
                },
            );
            self.resources = Some(resources);
            return Err(LocalAuthorityError::StaleGeneration {
                repo_id: self.repo_id,
                expected: self.generation,
                actual: generation,
            });
        }
        slots.insert(
            self.repo_id,
            RepoAuthoritySlot::Quiescing {
                generation,
                resources: None,
                leases: 0,
            },
        );
        drop(slots);
        drop(slot_resources);

        let resources = match Arc::try_unwrap(resources) {
            Ok(resources) => resources,
            Err(resources) => {
                self.restore_close_race(resources)?;
                return Err(LocalAuthorityError::Busy(self.repo_id));
            }
        };
        let RepoAuthorityResources {
            db,
            db_witness,
            authority_lock,
            lock_path: _,
            db_path,
        } = resources;
        drop(db);
        drop(db_witness);

        let mut slots = match self.inner.slots.lock() {
            Ok(slots) => slots,
            Err(_) => {
                // The durable cut already happened and the DB is closed. Keep
                // the process-level exclusion forever rather than reopening.
                std::mem::forget(authority_lock);
                self.settled = true;
                return Err(LocalAuthorityError::Poisoned);
            }
        };
        let exact_committed_reservation = matches!(
            slots.get(&self.repo_id),
            Some(RepoAuthoritySlot::Quiescing {
                generation,
                resources: None,
                leases: 0,
            }) if *generation == self.generation
        );
        if !exact_committed_reservation {
            std::mem::forget(authority_lock);
            self.settled = true;
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} lost its committed cleanup reservation",
                self.repo_id
            )));
        }
        slots.insert(
            self.repo_id,
            RepoAuthoritySlot::CommittedCleanup {
                generation: self.generation,
                authority_lock,
                db_path: db_path.clone(),
            },
        );
        self.settled = true;
        Ok(RepoAuthorityCleanupGuard {
            inner: self.inner.clone(),
            db_path,
            repo_id: self.repo_id,
            generation: self.generation,
            settled: false,
        })
    }

    fn restore_close_race(
        &mut self,
        resources: Arc<RepoAuthorityResources>,
    ) -> Result<(), LocalAuthorityError> {
        let mut slots = match self.inner.slots.lock() {
            Ok(slots) => slots,
            Err(_) => {
                self.resources = Some(resources);
                return Err(LocalAuthorityError::Poisoned);
            }
        };
        match slots.get_mut(&self.repo_id) {
            Some(RepoAuthoritySlot::Quiescing {
                generation,
                resources: slot_resources @ None,
                leases: 0,
            }) if *generation == self.generation => {
                *slot_resources = Some(resources.clone());
                self.resources = Some(resources);
                Ok(())
            }
            _ => {
                self.resources = Some(resources);
                Err(LocalAuthorityError::Invariant(format!(
                    "RepoId {} lost its close-race rollback reservation",
                    self.repo_id
                )))
            }
        }
    }

    fn rollback_inner(&mut self) -> Result<(), LocalAuthorityError> {
        if self.settled {
            return Ok(());
        }
        let guard_resources = self.resources.take().ok_or_else(|| {
            LocalAuthorityError::Invariant(format!(
                "RepoId {} quiesce rollback has no resources",
                self.repo_id
            ))
        })?;
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        let Some(slot) = slots.remove(&self.repo_id) else {
            self.resources = Some(guard_resources);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} quiescing slot disappeared during rollback",
                self.repo_id
            )));
        };
        let RepoAuthoritySlot::Quiescing {
            generation,
            resources: Some(slot_resources),
            leases: 0,
        } = slot
        else {
            slots.insert(self.repo_id, slot);
            self.resources = Some(guard_resources);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} is not rollback-capable quiescing authority",
                self.repo_id
            )));
        };
        if generation != self.generation || !Arc::ptr_eq(&slot_resources, &guard_resources) {
            slots.insert(
                self.repo_id,
                RepoAuthoritySlot::Quiescing {
                    generation,
                    resources: Some(slot_resources),
                    leases: 0,
                },
            );
            self.resources = Some(guard_resources);
            return Err(LocalAuthorityError::StaleGeneration {
                repo_id: self.repo_id,
                expected: self.generation,
                actual: generation,
            });
        }
        drop(guard_resources);
        slots.insert(
            self.repo_id,
            RepoAuthoritySlot::Active {
                generation,
                resources: slot_resources,
                leases: 0,
            },
        );
        self.settled = true;
        Ok(())
    }
}

impl Drop for RepoAuthorityQuiesceGuard {
    fn drop(&mut self) {
        if !self.settled
            && let Err(error) = self.rollback_inner()
        {
            tracing::error!(repo_id = %self.repo_id, %error, "failed to roll back local authority quiesce guard");
        }
    }
}

#[allow(dead_code)] // Consumed by the approved R4 removal coordinator.
impl RepoAuthorityCleanupGuard {
    pub(crate) fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Marks the authority slot retired only after owner-specific cleanup and
    /// exact catalog retirement have completed.
    pub(crate) fn complete(mut self) -> Result<(), LocalAuthorityError> {
        self.complete_inner()
    }

    fn complete_inner(&mut self) -> Result<(), LocalAuthorityError> {
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
            db_path,
        } = slot
        else {
            slots.insert(self.repo_id, slot);
            return Err(LocalAuthorityError::Invariant(format!(
                "RepoId {} is not committed-cleanup authority",
                self.repo_id
            )));
        };
        if generation != self.generation || db_path != self.db_path {
            slots.insert(
                self.repo_id,
                RepoAuthoritySlot::CommittedCleanup {
                    generation,
                    authority_lock,
                    db_path,
                },
            );
            return Err(LocalAuthorityError::StaleGeneration {
                repo_id: self.repo_id,
                expected: self.generation,
                actual: generation,
            });
        }
        slots.insert(
            self.repo_id,
            RepoAuthoritySlot::Retired {
                prior_generation: generation,
            },
        );
        drop(slots);
        drop(authority_lock);
        self.settled = true;
        Ok(())
    }
}

impl Drop for RepoAuthorityCleanupGuard {
    fn drop(&mut self) {
        if !self.settled {
            // A committed cleanup may only release its owner lock after the
            // owner-specific receipt and exact tombstone retirement succeed.
            // Leaving the slot untouched is intentionally fail-closed.
            tracing::error!(repo_id = %self.repo_id, generation = self.generation, "committed local authority cleanup guard dropped before completion; owner lock retained");
        }
    }
}
