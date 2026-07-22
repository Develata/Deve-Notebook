//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Owner-prepared Retired reincarnation and guarded authority activation.

use super::{LocalAuthorityError, LocalAuthorityRuntime, RepoAuthoritySlot};
use crate::models::RepoId;
use redb::Database;
use std::sync::Arc;
use uuid::Uuid;

/// Non-clone capability for an initialized authority that has not crossed the
/// durable catalog-membership cut.
pub struct PreparedRepoAuthority {
    pub(super) inner: Arc<super::LocalAuthorityInner>,
    pub(super) resources: Arc<super::RepoAuthorityResources>,
    pub(super) reservation_id: Uuid,
    pub(super) repo_id: RepoId,
    pub(super) generation: u64,
    pub(super) origin: PreparedRepoAuthorityOrigin,
    pub(super) settled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreparedRepoAuthorityOrigin {
    New,
    Reopening,
}

#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct PreparedAuthorityIdentity {
    pub(crate) database: crate::utils::fs::HostPathIdentity,
    pub(crate) authority_lock: crate::utils::fs::HostPathIdentity,
}

impl PreparedAuthorityIdentity {
    pub(crate) fn database(&self) -> &crate::utils::fs::HostPathIdentity {
        &self.database
    }

    pub(crate) fn authority_lock(&self) -> &crate::utils::fs::HostPathIdentity {
        &self.authority_lock
    }
}

impl LocalAuthorityRuntime {
    pub(crate) fn prepare_retired_repo_initialized<F>(
        &self,
        repo_id: RepoId,
        initialize: F,
    ) -> Result<PreparedRepoAuthority, LocalAuthorityError>
    where
        F: FnOnce(&Database) -> Result<(), LocalAuthorityError>,
    {
        let reservation_id = Uuid::new_v4();
        let (generation, expected_lock_identity, removed_database_identity) = {
            let mut slots = self
                .inner
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            let Some(RepoAuthoritySlot::Retired {
                prior_generation,
                expected_lock_identity,
                removed_database_identity,
            }) = slots.get(&repo_id)
            else {
                return match slots.get(&repo_id) {
                    Some(_) => Err(LocalAuthorityError::Busy(repo_id)),
                    None => Err(LocalAuthorityError::NotAdmitted(repo_id)),
                };
            };
            let prior_generation = *prior_generation;
            let expected_lock_identity = expected_lock_identity.clone();
            let removed_database_identity = removed_database_identity.clone();
            let Some(generation) = prior_generation.checked_add(1) else {
                slots.insert(
                    repo_id,
                    RepoAuthoritySlot::RepairRequired {
                        generation: prior_generation,
                    },
                );
                return Err(LocalAuthorityError::Invariant(format!(
                    "local authority generation exhausted for RepoId {repo_id}"
                )));
            };
            slots.insert(
                repo_id,
                RepoAuthoritySlot::Reopening {
                    reservation_id,
                    generation,
                    expected_lock_identity: expected_lock_identity.clone(),
                    removed_database_identity: removed_database_identity.clone(),
                    authority_lock: None,
                    resources: None,
                },
            );
            (
                generation,
                expected_lock_identity,
                removed_database_identity,
            )
        };
        let mut reopening = super::reservation::ReopeningReservation::new(
            &self.inner,
            repo_id,
            reservation_id,
            generation,
        );

        let authority_lock = match super::resource::open_reopening_lock(
            &self.inner.ledger_dir,
            repo_id,
            &expected_lock_identity,
            &removed_database_identity,
        ) {
            Ok(authority_lock) => authority_lock,
            Err(_) => return Err(LocalAuthorityError::RepairRequired(repo_id)),
        };
        reopening.attach_lock(&authority_lock)?;
        let resources = super::resource::create_reopening_resources(
            &self.inner.ledger_dir,
            repo_id,
            &expected_lock_identity,
            &removed_database_identity,
            authority_lock,
        )?;
        let resources = Arc::new(resources);
        reopening.attach_resources(&resources)?;
        initialize(resources.db())?;
        super::resource::validate_resource_identity(&resources)?;

        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        if !matches!(
            slots.get(&repo_id),
            Some(RepoAuthoritySlot::Reopening {
                reservation_id: current,
                generation: current_generation,
                expected_lock_identity: current_lock,
                removed_database_identity: current_database,
                authority_lock: Some(current_authority_lock),
                resources: Some(current_resources),
            }) if *current == reservation_id
                && *current_generation == generation
                && *current_lock == expected_lock_identity
                && *current_database == removed_database_identity
                && Arc::ptr_eq(current_authority_lock, &resources.authority_lock)
                && Arc::ptr_eq(current_resources, &resources)
        ) {
            return Err(LocalAuthorityError::Invariant(format!(
                "local authority reopening reservation changed for RepoId {repo_id}"
            )));
        }
        slots.insert(
            repo_id,
            RepoAuthoritySlot::ReopeningPrepared {
                reservation_id,
                generation,
                expected_lock_identity,
                removed_database_identity,
                resources: resources.clone(),
            },
        );
        reopening.settle_after_transition();
        Ok(PreparedRepoAuthority {
            inner: self.inner.clone(),
            resources,
            reservation_id,
            repo_id,
            generation,
            origin: PreparedRepoAuthorityOrigin::Reopening,
            settled: false,
        })
    }
}

impl PreparedRepoAuthority {
    pub(crate) fn db(&self) -> &Database {
        self.resources.db()
    }

    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(crate) fn identity_observation(
        &self,
    ) -> Result<super::PreparedAuthorityIdentity, LocalAuthorityError> {
        super::resource::validate_resource_identity(&self.resources)?;
        let database = crate::utils::fs::HostPathIdentity::capture(
            &self.resources.db_path,
            crate::utils::fs::HostPathKind::RegularFile,
        )?;
        let authority_lock = crate::utils::fs::HostPathIdentity::capture(
            &self.resources.lock_path,
            crate::utils::fs::HostPathKind::RegularFile,
        )?;
        crate::utils::fs::ensure_open_file_matches_identity(
            &self.resources.db_witness,
            &database,
            "prepared local authority database",
        )?;
        crate::utils::fs::ensure_open_file_matches_identity(
            &self.resources.authority_lock,
            &authority_lock,
            "prepared local authority lock",
        )?;
        Ok(super::PreparedAuthorityIdentity {
            database,
            authority_lock,
        })
    }

    #[cfg(test)]
    pub(crate) fn activate_for_test(
        self,
        owner: &LocalAuthorityRuntime,
    ) -> Result<(), LocalAuthorityError> {
        self.activate_slot(owner)
    }

    fn activate_slot(mut self, owner: &LocalAuthorityRuntime) -> Result<(), LocalAuthorityError> {
        if !Arc::ptr_eq(&self.inner, &owner.inner) {
            return Err(LocalAuthorityError::NotAdmitted(self.repo_id));
        }
        super::resource::validate_resource_identity(&self.resources)?;
        let reopening_identity = if self.origin == PreparedRepoAuthorityOrigin::Reopening {
            let slots = self
                .inner
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            match slots.get(&self.repo_id) {
                Some(RepoAuthoritySlot::ReopeningPrepared {
                    reservation_id,
                    generation,
                    expected_lock_identity,
                    removed_database_identity,
                    resources,
                }) if *reservation_id == self.reservation_id
                    && *generation == self.generation
                    && Arc::ptr_eq(resources, &self.resources) =>
                {
                    Some((
                        expected_lock_identity.clone(),
                        removed_database_identity.clone(),
                    ))
                }
                _ => return Err(LocalAuthorityError::Busy(self.repo_id)),
            }
        } else {
            None
        };
        if let Some((expected_lock_identity, _)) = &reopening_identity
            && crate::utils::fs::ensure_open_file_matches_identity(
                &self.resources.authority_lock,
                expected_lock_identity,
                "reopened local authority lock",
            )
            .is_err()
        {
            let mut slots = self
                .inner
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            if matches!(
                slots.get(&self.repo_id),
                Some(RepoAuthoritySlot::ReopeningPrepared {
                    reservation_id,
                    generation,
                    resources,
                    ..
                }) if *reservation_id == self.reservation_id
                    && *generation == self.generation
                    && Arc::ptr_eq(resources, &self.resources)
            ) {
                let Some(RepoAuthoritySlot::ReopeningPrepared {
                    generation,
                    expected_lock_identity,
                    removed_database_identity,
                    resources,
                    ..
                }) = slots.remove(&self.repo_id)
                else {
                    unreachable!("exact reopening slot was matched above")
                };
                slots.insert(
                    self.repo_id,
                    RepoAuthoritySlot::ReopeningRepairRequired {
                        generation,
                        expected_lock_identity,
                        removed_database_identity,
                        authority_lock: Some(resources.authority_lock.clone()),
                        resources: Some(resources),
                    },
                );
                self.settled = true;
            }
            return Err(LocalAuthorityError::RepairRequired(self.repo_id));
        }
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        let Some(slot) = slots.remove(&self.repo_id) else {
            return Err(LocalAuthorityError::Invariant(format!(
                "prepared local authority disappeared for RepoId {}",
                self.repo_id
            )));
        };
        let (reservation_id, generation, resources, reopening_identity) = match (self.origin, slot)
        {
            (
                PreparedRepoAuthorityOrigin::New,
                RepoAuthoritySlot::Preparing {
                    reservation_id,
                    generation,
                    resources,
                },
            ) => (reservation_id, generation, resources, None),
            (
                PreparedRepoAuthorityOrigin::Reopening,
                RepoAuthoritySlot::ReopeningPrepared {
                    reservation_id,
                    generation,
                    expected_lock_identity,
                    removed_database_identity,
                    resources,
                },
            ) => (
                reservation_id,
                generation,
                resources,
                Some((expected_lock_identity, removed_database_identity)),
            ),
            (_, slot) => {
                slots.insert(self.repo_id, slot);
                return Err(LocalAuthorityError::Busy(self.repo_id));
            }
        };
        if reservation_id != self.reservation_id
            || generation != self.generation
            || !Arc::ptr_eq(&resources, &self.resources)
        {
            match self.origin {
                PreparedRepoAuthorityOrigin::New => {
                    slots.insert(
                        self.repo_id,
                        RepoAuthoritySlot::Preparing {
                            reservation_id,
                            generation,
                            resources,
                        },
                    );
                }
                PreparedRepoAuthorityOrigin::Reopening => {
                    let (expected_lock_identity, removed_database_identity) =
                        reopening_identity.expect("reopening slot carries frozen identities");
                    slots.insert(
                        self.repo_id,
                        RepoAuthoritySlot::ReopeningRepairRequired {
                            generation,
                            expected_lock_identity,
                            removed_database_identity,
                            authority_lock: Some(resources.authority_lock.clone()),
                            resources: Some(resources),
                        },
                    );
                }
            }
            return Err(LocalAuthorityError::Invariant(format!(
                "prepared local authority identity changed for RepoId {}",
                self.repo_id
            )));
        }
        slots.insert(
            self.repo_id,
            RepoAuthoritySlot::Active {
                generation,
                resources,
                leases: 0,
            },
        );
        if owner.primary_repo.get().is_none() {
            let _ = owner
                .primary_repo
                .get_or_init(|| super::PrimaryRepoBinding::new(self.repo_id));
        }
        self.settled = true;
        Ok(())
    }

    pub(crate) fn activate_composed_guarded(
        self,
        owner: &LocalAuthorityRuntime,
        identity: &super::PreparedAuthorityIdentity,
    ) -> Result<(), LocalAuthorityError> {
        if !Arc::ptr_eq(&self.inner, &owner.inner) {
            return Err(LocalAuthorityError::NotAdmitted(self.repo_id));
        }
        crate::utils::fs::ensure_open_file_matches_identity(
            &self.resources.db_witness,
            identity.database(),
            "prepared activation database",
        )?;
        crate::utils::fs::ensure_open_file_matches_identity(
            &self.resources.authority_lock,
            identity.authority_lock(),
            "prepared activation lock",
        )?;
        // `activate_slot` repeats the ordinary resource/path validation before
        // its short exact CAS. The caller owns locator and catalog guards for
        // the entire call, so no project-owned identity mutation can interleave.
        self.activate_slot(owner)
    }
}

impl Drop for PreparedRepoAuthority {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        let Ok(mut slots) = self.inner.slots.lock() else {
            tracing::error!(repo_id = %self.repo_id, "prepared local authority drop could not seal poisoned slot");
            return;
        };
        let exact = match (self.origin, slots.get(&self.repo_id)) {
            (
                PreparedRepoAuthorityOrigin::New,
                Some(RepoAuthoritySlot::Preparing {
                    reservation_id,
                    generation,
                    resources,
                }),
            )
            | (
                PreparedRepoAuthorityOrigin::Reopening,
                Some(RepoAuthoritySlot::ReopeningPrepared {
                    reservation_id,
                    generation,
                    resources,
                    ..
                }),
            ) => {
                *reservation_id == self.reservation_id
                    && *generation == self.generation
                    && Arc::ptr_eq(resources, &self.resources)
            }
            _ => false,
        };
        if exact {
            match self.origin {
                PreparedRepoAuthorityOrigin::New => {
                    slots.insert(
                        self.repo_id,
                        RepoAuthoritySlot::RepairRequired {
                            generation: self.generation,
                        },
                    );
                }
                PreparedRepoAuthorityOrigin::Reopening => {
                    let Some(RepoAuthoritySlot::ReopeningPrepared {
                        expected_lock_identity,
                        removed_database_identity,
                        resources,
                        ..
                    }) = slots.remove(&self.repo_id)
                    else {
                        unreachable!("exact reopening slot was matched above")
                    };
                    slots.insert(
                        self.repo_id,
                        RepoAuthoritySlot::ReopeningRepairRequired {
                            generation: self.generation,
                            expected_lock_identity,
                            removed_database_identity,
                            authority_lock: Some(resources.authority_lock.clone()),
                            resources: Some(resources),
                        },
                    );
                }
            }
        } else {
            tracing::error!(repo_id = %self.repo_id, "prepared local authority drop did not match its slot");
        }
    }
}
