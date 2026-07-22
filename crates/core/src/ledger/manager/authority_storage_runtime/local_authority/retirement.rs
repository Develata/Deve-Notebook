//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Pre-commit quiescence and post-commit authority retirement capabilities.

mod cleanup_drop;
mod recovery;

use super::checkpoint::RepoAuthorityDatabaseCheckpointState;
use super::{
    LocalAuthorityError, RepoAuthorityCleanupGuard, RepoAuthorityDatabaseCheckpoint,
    RepoAuthorityQuiesceGuard, RepoAuthorityRemovalSnapshot, RepoAuthorityResources,
    RepoAuthoritySlot,
};
use crate::models::RepoId;
use crate::utils::fs::{HostPathIdentity, HostPathKind};
use std::sync::Arc;

#[allow(dead_code)] // Consumed by the approved R3/R4 removal coordinator.
impl RepoAuthorityQuiesceGuard {
    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Re-opens the admission gate without changing its generation.
    pub fn rollback(mut self) -> Result<(), LocalAuthorityError> {
        self.rollback_inner()
    }

    /// Crosses the irreversible authority cut after the caller has durably
    /// committed the removal tombstone. The database closes here, while the
    /// persistent owner lock moves into the slot itself.
    pub fn into_committed_cleanup(
        mut self,
    ) -> Result<RepoAuthorityCleanupGuard, LocalAuthorityError> {
        // This capability is callable only after the durable Removed cut. From
        // this point every error must remain fail-closed in Quiescing or
        // CommittedCleanup; Drop must never reopen ordinary admission.
        self.settled = true;
        let current_resources = self.resources.as_ref().ok_or_else(|| {
            LocalAuthorityError::Invariant(format!(
                "RepoId {} quiesce guard has no resources",
                self.repo_id
            ))
        })?;
        super::resource::validate_resource_identity(current_resources)?;
        let expected_lock_identity =
            HostPathIdentity::capture(&current_resources.lock_path, HostPathKind::RegularFile)?;
        let removed_database_identity =
            HostPathIdentity::capture(&current_resources.db_path, HostPathKind::RegularFile)?;
        crate::utils::fs::ensure_open_file_matches_identity(
            &current_resources.authority_lock,
            &expected_lock_identity,
            "local authority retirement lock",
        )?;
        crate::utils::fs::ensure_open_file_matches_identity(
            &current_resources.db_witness,
            &removed_database_identity,
            "local authority retirement database",
        )?;
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
                expected_lock_identity,
                removed_database_identity,
                db_path: db_path.clone(),
                cleanup_capability_issued: true,
            },
        );
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
    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    #[cfg(test)]
    pub(crate) fn db_path(&self) -> &std::path::Path {
        &self.db_path
    }

    pub fn advance_database_cleanup(
        &mut self,
        snapshot: &RepoAuthorityRemovalSnapshot,
        checkpoint: &RepoAuthorityDatabaseCheckpoint,
    ) -> Result<RepoAuthorityDatabaseCheckpoint, LocalAuthorityError> {
        if self.settled
            || snapshot.repo_id != self.repo_id
            || snapshot.generation != self.generation
            || snapshot.database.kind() != crate::utils::fs::HostPathKind::RegularFile
            || snapshot.database.path() != self.db_path
        {
            return Err(LocalAuthorityError::CleanupIdentityChanged(self.repo_id));
        }
        {
            let slots = self
                .inner
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            if !matches!(
                slots.get(&self.repo_id),
                Some(RepoAuthoritySlot::CommittedCleanup { generation, db_path, .. })
                    if *generation == self.generation && *db_path == self.db_path
            ) {
                return Err(LocalAuthorityError::StaleGeneration {
                    repo_id: self.repo_id,
                    expected: self.generation,
                    actual: slot_generation(slots.get(&self.repo_id)).unwrap_or(0),
                });
            }
        }
        let state = match &checkpoint.state {
            RepoAuthorityDatabaseCheckpointState::Prepared => {
                RepoAuthorityDatabaseCheckpointState::DatabaseQuarantined {
                    database: snapshot.database_quarantine.cut()?,
                }
            }
            RepoAuthorityDatabaseCheckpointState::DatabaseQuarantined { database } => {
                database.delete()?;
                RepoAuthorityDatabaseCheckpointState::DatabaseDeleted {
                    database: database.clone(),
                }
            }
            RepoAuthorityDatabaseCheckpointState::DatabaseDeleted { database } => {
                if !database.is_deleted()? {
                    return Err(LocalAuthorityError::CleanupIdentityChanged(self.repo_id));
                }
                RepoAuthorityDatabaseCheckpointState::DatabaseDeleted {
                    database: database.clone(),
                }
            }
        };
        Ok(RepoAuthorityDatabaseCheckpoint { state })
    }

    pub fn verify_database_cleanup_complete(
        &self,
        snapshot: &RepoAuthorityRemovalSnapshot,
        checkpoint: &RepoAuthorityDatabaseCheckpoint,
    ) -> Result<(), LocalAuthorityError> {
        if self.settled
            || snapshot.repo_id != self.repo_id
            || snapshot.generation != self.generation
        {
            return Err(LocalAuthorityError::CleanupIdentityChanged(self.repo_id));
        }
        let RepoAuthorityDatabaseCheckpointState::DatabaseDeleted { database } = &checkpoint.state
        else {
            return Err(LocalAuthorityError::CleanupIdentityChanged(self.repo_id));
        };
        if !database.belongs_to(&snapshot.database_quarantine) || !database.is_deleted()? {
            return Err(LocalAuthorityError::CleanupIdentityChanged(self.repo_id));
        }
        Ok(())
    }

    /// Called only by the composed `RepoManager` after it has verified both
    /// the owner-issued DB checkpoint and exact catalog tombstone retirement.
    pub(crate) fn complete_inner(&mut self) -> Result<(), LocalAuthorityError> {
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
        // Make `Retired` observable only after the last cleanup lock handle
        // has been released. Holding the slot mutex across this in-memory drop
        // prevents a valid readmission from seeing Retired and racing the old
        // handle's unlock.
        drop(authority_lock);
        slots.insert(
            self.repo_id,
            RepoAuthoritySlot::Retired {
                prior_generation: generation,
                expected_lock_identity,
                removed_database_identity,
            },
        );
        drop(slots);
        self.settled = true;
        Ok(())
    }
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
        | RepoAuthoritySlot::CommittedCleanup { generation, .. } => Some(*generation),
        RepoAuthoritySlot::Retired {
            prior_generation, ..
        } => Some(*prior_generation),
    }
}
