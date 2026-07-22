//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!
//! Existing-authority admission and generation-bound lease choreography.

use super::{
    BoundRepoAuthority, LocalAuthorityDiscovery, LocalAuthorityError, LocalAuthorityInner,
    LocalAuthorityRuntime, RepoAuthorityLease, RepoAuthoritySlot,
};
use crate::models::RepoId;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use uuid::Uuid;

pub(super) struct PrimaryRepoBinding {
    pub(super) repo_id: RepoId,
    pub(super) execution_name: String,
}

impl PrimaryRepoBinding {
    pub(super) fn new(repo_id: RepoId) -> Self {
        Self {
            repo_id,
            execution_name: repo_id.to_string(),
        }
    }
}

impl LocalAuthorityDiscovery {
    pub(crate) fn new(ledger_dir: &Path) -> Self {
        Self {
            inner: Arc::new(LocalAuthorityInner {
                ledger_dir: ledger_dir.to_path_buf(),
                slots: Mutex::new(HashMap::new()),
                lease_released: Condvar::new(),
            }),
        }
    }

    pub(crate) fn lease(&self, repo_id: RepoId) -> Result<RepoAuthorityLease, LocalAuthorityError> {
        match lease_from_inner(&self.inner, repo_id) {
            Err(LocalAuthorityError::NotAdmitted(_)) => {
                admit_existing_from_inner(&self.inner, repo_id)
            }
            result => result,
        }
    }
}

impl BoundRepoAuthority {
    pub(super) fn new(
        runtime: &Arc<LocalAuthorityInner>,
        catalog: crate::ledger::CatalogMembershipRuntime,
        membership: crate::ledger::CatalogMembershipToken,
    ) -> Self {
        Self {
            runtime: Arc::downgrade(runtime),
            repo_id: membership.repo_id(),
            catalog,
            membership,
        }
    }

    pub(crate) fn lease(&self) -> Result<RepoAuthorityLease, LocalAuthorityError> {
        self.catalog
            .revalidate(&self.membership)
            .map_err(|_| LocalAuthorityError::NotAdmitted(self.repo_id))?;
        let runtime = self.runtime.upgrade().ok_or_else(|| {
            LocalAuthorityError::Invariant(format!(
                "local authority owner was dropped for RepoId {}",
                self.repo_id
            ))
        })?;
        let lease = match lease_from_inner(&runtime, self.repo_id) {
            Err(LocalAuthorityError::NotAdmitted(_)) => {
                admit_existing_from_inner(&runtime, self.repo_id)?
            }
            result => result?,
        };
        self.catalog
            .revalidate(&self.membership)
            .map_err(|_| LocalAuthorityError::NotAdmitted(self.repo_id))?;
        Ok(lease)
    }
}

impl LocalAuthorityRuntime {
    pub(crate) fn bind(
        &self,
        catalog: crate::ledger::CatalogMembershipRuntime,
        membership: crate::ledger::CatalogMembershipToken,
    ) -> BoundRepoAuthority {
        BoundRepoAuthority::new(&self.inner, catalog, membership)
    }
}

pub(super) fn lease_from_inner(
    inner: &Arc<LocalAuthorityInner>,
    repo_id: RepoId,
) -> Result<RepoAuthorityLease, LocalAuthorityError> {
    let lease = {
        let mut slots = inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        match slots.get_mut(&repo_id) {
            Some(RepoAuthoritySlot::Active {
                generation,
                resources,
                leases,
            }) => {
                *leases = leases.checked_add(1).ok_or_else(|| {
                    LocalAuthorityError::Invariant(format!(
                        "local authority lease count exhausted for RepoId {repo_id}"
                    ))
                })?;
                RepoAuthorityLease {
                    runtime: Arc::downgrade(inner),
                    resources: resources.clone(),
                    repo_id,
                    generation: *generation,
                }
            }
            Some(RepoAuthoritySlot::Quiescing { .. })
            | Some(RepoAuthoritySlot::CommittedCleanup { .. }) => {
                return Err(LocalAuthorityError::Quiescing(repo_id));
            }
            Some(RepoAuthoritySlot::Opening { .. }) | Some(RepoAuthoritySlot::Preparing { .. }) => {
                return Err(LocalAuthorityError::Busy(repo_id));
            }
            Some(RepoAuthoritySlot::Reopening { .. })
            | Some(RepoAuthoritySlot::ReopeningPrepared { .. }) => {
                return Err(LocalAuthorityError::Busy(repo_id));
            }
            Some(RepoAuthoritySlot::RepairRequired { .. }) => {
                return Err(LocalAuthorityError::RepairRequired(repo_id));
            }
            Some(RepoAuthoritySlot::ReopeningRepairRequired { .. }) => {
                return Err(LocalAuthorityError::RepairRequired(repo_id));
            }
            Some(RepoAuthoritySlot::Retired { .. }) => {
                return Err(LocalAuthorityError::Retired(repo_id));
            }
            None => return Err(LocalAuthorityError::NotAdmitted(repo_id)),
        }
    };

    // Re-check both pathname identities for every admission. Dropping this
    // lease on failure decrements the same mutex-protected lease counter.
    super::resource::validate_resource_identity(&lease.resources)?;
    Ok(lease)
}

pub(super) fn admit_existing_from_inner(
    inner: &Arc<LocalAuthorityInner>,
    repo_id: RepoId,
) -> Result<RepoAuthorityLease, LocalAuthorityError> {
    admit_existing_from_inner_with_hook(inner, repo_id, || {})
}

fn admit_existing_from_inner_with_hook(
    inner: &Arc<LocalAuthorityInner>,
    repo_id: RepoId,
    before_open: impl FnOnce(),
) -> Result<RepoAuthorityLease, LocalAuthorityError> {
    let reservation_id = Uuid::new_v4();
    {
        let mut slots = inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        match slots.get(&repo_id) {
            Some(RepoAuthoritySlot::Active { .. }) => {
                drop(slots);
                return lease_from_inner(inner, repo_id);
            }
            Some(RepoAuthoritySlot::Opening { .. })
            | Some(RepoAuthoritySlot::Reopening { .. })
            | Some(RepoAuthoritySlot::Preparing { .. })
            | Some(RepoAuthoritySlot::ReopeningPrepared { .. }) => {
                return Err(LocalAuthorityError::Busy(repo_id));
            }
            Some(RepoAuthoritySlot::Quiescing { .. })
            | Some(RepoAuthoritySlot::CommittedCleanup { .. }) => {
                return Err(LocalAuthorityError::Quiescing(repo_id));
            }
            Some(RepoAuthoritySlot::RepairRequired { .. }) => {
                return Err(LocalAuthorityError::RepairRequired(repo_id));
            }
            Some(RepoAuthoritySlot::ReopeningRepairRequired { .. }) => {
                return Err(LocalAuthorityError::RepairRequired(repo_id));
            }
            Some(RepoAuthoritySlot::Retired { .. }) => {
                return Err(LocalAuthorityError::Retired(repo_id));
            }
            None => {
                slots.insert(repo_id, RepoAuthoritySlot::Opening { reservation_id });
            }
        }
    }
    let mut opening = super::reservation::OpeningReservation::new(inner, repo_id, reservation_id);

    before_open();
    let opened =
        super::resource::open_resources(&inner.ledger_dir, repo_id, false).and_then(|resources| {
            super::resource::validate_existing(resources.db_path.as_path(), resources.db())?;
            super::resource::validate_resource_identity(&resources)?;
            Ok(Arc::new(resources))
        });
    let mut slots = inner
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
            "local authority admission reservation changed for RepoId {repo_id}"
        )));
    }
    match opened {
        Ok(resources) => {
            slots.insert(
                repo_id,
                RepoAuthoritySlot::Active {
                    generation: 1,
                    resources: resources.clone(),
                    leases: 1,
                },
            );
            let lease = RepoAuthorityLease {
                runtime: Arc::downgrade(inner),
                resources,
                repo_id,
                generation: 1,
            };
            opening.settle_after_transition();
            Ok(lease)
        }
        Err(LocalAuthorityError::Busy(_)) => {
            slots.remove(&repo_id);
            opening.settle_after_transition();
            Err(LocalAuthorityError::Busy(repo_id))
        }
        Err(error) => {
            slots.insert(repo_id, RepoAuthoritySlot::RepairRequired { generation: 1 });
            opening.settle_after_transition();
            // Seal the ambiguous incarnation for explicit repair, while
            // preserving the typed schema/authority failure that explains why
            // admission was rejected. The slot state and the returned
            // diagnostic serve different responsibilities.
            Err(error)
        }
    }
}

#[cfg(test)]
pub(super) fn admit_existing_with_hook_for_test(
    inner: &Arc<LocalAuthorityInner>,
    repo_id: RepoId,
    before_open: impl FnOnce(),
) -> Result<RepoAuthorityLease, LocalAuthorityError> {
    admit_existing_from_inner_with_hook(inner, repo_id, before_open)
}
