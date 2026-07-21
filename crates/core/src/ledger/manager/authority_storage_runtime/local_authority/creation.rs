//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Membership-bound creation admission for a new local authority.

use super::{LocalAuthorityError, LocalAuthorityRuntime, PreparedRepoAuthority, RepoAuthoritySlot};
use crate::ledger::{CatalogMembershipRuntime, CatalogMembershipToken};
use crate::models::RepoId;
use redb::Database;
use std::sync::Arc;
use uuid::Uuid;

impl LocalAuthorityRuntime {
    pub(crate) fn create_repo_initialized<F>(
        &self,
        repo_id: RepoId,
        initialize: F,
    ) -> Result<PreparedRepoAuthority, LocalAuthorityError>
    where
        F: FnOnce(&Database) -> Result<(), LocalAuthorityError>,
    {
        let reservation_id = Uuid::new_v4();
        {
            let mut slots = self
                .inner
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            if slots.contains_key(&repo_id) {
                return Err(LocalAuthorityError::Busy(repo_id));
            }
            slots.insert(repo_id, RepoAuthoritySlot::Opening { reservation_id });
        }
        let mut opening =
            super::reservation::OpeningReservation::new(&self.inner, repo_id, reservation_id);

        let opened = super::resource::open_resources(&self.inner.ledger_dir, repo_id, true);
        let initialized = match opened {
            Ok(resources) => {
                opening.require_repair_on_drop();
                match initialize(resources.db()) {
                    Ok(()) => Ok(Arc::new(resources)),
                    Err(error) => {
                        drop(resources);
                        Err((error, true))
                    }
                }
            }
            Err(error) => {
                let db_path = self
                    .inner
                    .ledger_dir
                    .join("local")
                    .join(format!("{repo_id}.redb"));
                let residual = db_path.try_exists().unwrap_or(true);
                Err((error, residual))
            }
        };
        let mut slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        if !matches!(
            slots.get(&repo_id),
            Some(RepoAuthoritySlot::Opening {
                reservation_id: current,
            }) if *current == reservation_id
        ) {
            return Err(LocalAuthorityError::Invariant(format!(
                "local authority create reservation changed for RepoId {repo_id}"
            )));
        }
        match initialized {
            Ok(resources) => {
                slots.insert(
                    repo_id,
                    RepoAuthoritySlot::Preparing {
                        reservation_id,
                        generation: 1,
                        resources: resources.clone(),
                    },
                );
                Ok(PreparedRepoAuthority {
                    inner: self.inner.clone(),
                    resources,
                    reservation_id,
                    repo_id,
                    generation: 1,
                    settled: false,
                })
            }
            Err((error, repair_required)) => {
                if repair_required {
                    slots.insert(repo_id, RepoAuthoritySlot::RepairRequired { generation: 1 });
                } else {
                    slots.remove(&repo_id);
                }
                Err(error)
            }
        }
        .inspect(|_| opening.settle_after_transition())
        .inspect_err(|_| opening.settle_after_transition())
    }
}

impl PreparedRepoAuthority {
    pub(crate) fn db(&self) -> &Database {
        self.resources.db()
    }

    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    /// Publishes the new authority only against a live token from the exact
    /// catalog runtime that committed this RepoId's membership.
    pub(crate) fn activate(
        self,
        owner: &LocalAuthorityRuntime,
        membership: &CatalogMembershipToken,
        catalog: &CatalogMembershipRuntime,
    ) -> Result<(), LocalAuthorityError> {
        if !Arc::ptr_eq(&self.inner, &owner.inner)
            || membership.repo_id() != self.repo_id
            || catalog.revalidate(membership).is_err()
        {
            return Err(LocalAuthorityError::NotAdmitted(self.repo_id));
        }
        self.activate_slot(owner)
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
        let RepoAuthoritySlot::Preparing {
            reservation_id,
            generation,
            resources,
        } = slot
        else {
            slots.insert(self.repo_id, slot);
            return Err(LocalAuthorityError::Busy(self.repo_id));
        };
        if reservation_id != self.reservation_id
            || generation != self.generation
            || !Arc::ptr_eq(&resources, &self.resources)
        {
            slots.insert(
                self.repo_id,
                RepoAuthoritySlot::Preparing {
                    reservation_id,
                    generation,
                    resources,
                },
            );
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
        let exact = matches!(
            slots.get(&self.repo_id),
            Some(RepoAuthoritySlot::Preparing {
                reservation_id,
                generation,
                resources,
            }) if *reservation_id == self.reservation_id
                && *generation == self.generation
                && Arc::ptr_eq(resources, &self.resources)
        );
        if exact {
            slots.insert(
                self.repo_id,
                RepoAuthoritySlot::RepairRequired {
                    generation: self.generation,
                },
            );
        } else {
            tracing::error!(repo_id = %self.repo_id, "prepared local authority drop did not match its slot");
        }
    }
}
