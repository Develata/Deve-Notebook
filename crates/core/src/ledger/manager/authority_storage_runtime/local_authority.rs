//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#local-repo-removal-contract
//!
//! Process-local owner of every local Redb authority handle.

mod admission;
mod creation;
mod drainage;
mod inspection;
mod reservation;
mod resource;
mod retirement;

use crate::models::RepoId;
use admission::{admit_existing_from_inner, lease_from_inner};
use redb::Database;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, Weak};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum LocalAuthorityError {
    #[error("local authority runtime state is poisoned")]
    Poisoned,
    #[error("local authority for RepoId {0} is busy")]
    Busy(RepoId),
    #[error("local authority for RepoId {0} is quiescing")]
    Quiescing(RepoId),
    #[error("local authority for RepoId {0} is retired")]
    Retired(RepoId),
    #[error("local authority for RepoId {0} is not admitted by current catalog membership")]
    NotAdmitted(RepoId),
    #[error("local authority for RepoId {0} requires explicit repair")]
    RepairRequired(RepoId),
    #[error(
        "local authority generation for RepoId {repo_id} is stale: expected {expected}, actual {actual}"
    )]
    StaleGeneration {
        repo_id: RepoId,
        expected: u64,
        actual: u64,
    },
    #[error("timed out while draining local authority leases for RepoId {0}")]
    DrainTimeout(RepoId),
    #[error("local authority invariant failed: {0}")]
    Invariant(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Database(#[from] redb::DatabaseError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

struct RepoAuthorityResources {
    db: Database,
    db_witness: File,
    authority_lock: File,
    lock_path: PathBuf,
    db_path: PathBuf,
}

#[allow(dead_code)] // R3/R4 consumes terminal generations outside test builds.
enum RepoAuthoritySlot {
    Opening {
        reservation_id: Uuid,
    },
    Preparing {
        reservation_id: Uuid,
        generation: u64,
        resources: Arc<RepoAuthorityResources>,
    },
    RepairRequired {
        generation: u64,
    },
    Active {
        generation: u64,
        resources: Arc<RepoAuthorityResources>,
        leases: usize,
    },
    Quiescing {
        generation: u64,
        resources: Option<Arc<RepoAuthorityResources>>,
        leases: usize,
    },
    CommittedCleanup {
        generation: u64,
        authority_lock: File,
        db_path: PathBuf,
    },
    Retired {
        prior_generation: u64,
    },
}

struct LocalAuthorityInner {
    ledger_dir: PathBuf,
    slots: Mutex<HashMap<RepoId, RepoAuthoritySlot>>,
    lease_released: Condvar,
}

/// The unique composition-root owner for local Redb handles.
///
/// This type is deliberately not `Clone`. Short-lived access is represented by
/// a non-clone [`RepoAuthorityLease`].
pub(crate) struct LocalAuthorityRuntime {
    primary_repo_id: RepoId,
    primary_repo_name: String,
    inner: Arc<LocalAuthorityInner>,
}

/// Bootstrap-only owner used while selecting an existing local RepoId.
///
/// Discovery leases use the same slot and persistent-lock implementation as
/// the composed runtime. Dropping this owner closes every probed database
/// before the selected runtime is composed.
pub(crate) struct LocalAuthorityDiscovery {
    inner: Arc<LocalAuthorityInner>,
}

#[derive(Clone)]
pub(crate) struct BoundRepoAuthority {
    runtime: Weak<LocalAuthorityInner>,
    catalog: crate::ledger::CatalogMembershipRuntime,
    membership: crate::ledger::CatalogMembershipToken,
    repo_id: RepoId,
}

pub struct RepoAuthorityLease {
    runtime: Weak<LocalAuthorityInner>,
    resources: Arc<RepoAuthorityResources>,
    repo_id: RepoId,
    generation: u64,
}

/// Non-clone capability for a newly initialized authority that has not yet
/// crossed the durable catalog-membership cut.
pub struct PreparedRepoAuthority {
    inner: Arc<LocalAuthorityInner>,
    resources: Arc<RepoAuthorityResources>,
    reservation_id: Uuid,
    repo_id: RepoId,
    generation: u64,
    settled: bool,
}

/// Exclusive pre-commit capability for a repo authority cut.
///
/// Dropping this value rolls the slot back to `Active`. Only the removal
/// coordinator may cross the durable commit cut with `into_committed_cleanup`.
#[allow(dead_code)] // Consumed by the approved R3/R4 removal coordinator.
pub(crate) struct RepoAuthorityQuiesceGuard {
    inner: Arc<LocalAuthorityInner>,
    resources: Option<Arc<RepoAuthorityResources>>,
    repo_id: RepoId,
    generation: u64,
    settled: bool,
}

/// Post-commit cleanup capability. The Redb handle is already closed while the
/// cross-process owner lock remains held.
#[allow(dead_code)] // Consumed by the approved R4 removal coordinator.
pub(crate) struct RepoAuthorityCleanupGuard {
    inner: Arc<LocalAuthorityInner>,
    db_path: PathBuf,
    repo_id: RepoId,
    generation: u64,
    settled: bool,
}

impl RepoAuthorityLease {
    pub fn db(&self) -> &Database {
        &self.resources.db
    }

    #[allow(dead_code)] // Bound into the R3 confirmation token.
    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    #[allow(dead_code)] // Bound into the R3 confirmation token.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    #[cfg(test)]
    pub(crate) fn db_path(&self) -> &Path {
        &self.resources.db_path
    }
}

impl Drop for RepoAuthorityLease {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.upgrade() {
            let released = runtime.slots.lock().ok().and_then(|mut slots| {
                let slot = slots.get_mut(&self.repo_id)?;
                let leases = match slot {
                    RepoAuthoritySlot::Active {
                        generation,
                        resources,
                        leases,
                    }
                    | RepoAuthoritySlot::Quiescing {
                        generation,
                        resources: Some(resources),
                        leases,
                    } if *generation == self.generation
                        && Arc::ptr_eq(resources, &self.resources) =>
                    {
                        leases
                    }
                    _ => return None,
                };
                *leases = leases.checked_sub(1)?;
                Some(())
            });
            if released.is_some() {
                runtime.lease_released.notify_all();
            } else {
                tracing::error!(repo_id = %self.repo_id, generation = self.generation, "local authority lease release did not match its slot");
            }
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoAuthoritySlotSnapshot {
    Opening,
    Preparing { generation: u64 },
    RepairRequired { generation: u64 },
    Active { generation: u64 },
    Quiescing { generation: u64 },
    CommittedCleanup { generation: u64 },
    Retired { prior_generation: u64 },
}

impl LocalAuthorityRuntime {
    pub(crate) fn prepare_new_initialized<F>(
        ledger_dir: &Path,
        primary_repo_id: RepoId,
        initialize: F,
    ) -> Result<(Self, PreparedRepoAuthority), LocalAuthorityError>
    where
        F: FnOnce(&Database) -> Result<(), LocalAuthorityError>,
    {
        let runtime = Self {
            primary_repo_id,
            primary_repo_name: primary_repo_id.to_string(),
            inner: Arc::new(LocalAuthorityInner {
                ledger_dir: ledger_dir.to_path_buf(),
                slots: Mutex::new(HashMap::new()),
                lease_released: Condvar::new(),
            }),
        };
        let prepared = runtime.create_repo_initialized(primary_repo_id, initialize)?;
        Ok((runtime, prepared))
    }

    pub(crate) fn open_existing(
        ledger_dir: &Path,
        primary_repo_id: RepoId,
    ) -> Result<Self, LocalAuthorityError> {
        let inner = Arc::new(LocalAuthorityInner {
            ledger_dir: ledger_dir.to_path_buf(),
            slots: Mutex::new(HashMap::new()),
            lease_released: Condvar::new(),
        });
        let lease = admit_existing_from_inner(&inner, primary_repo_id)?;
        drop(lease);
        Ok(Self {
            primary_repo_id,
            primary_repo_name: primary_repo_id.to_string(),
            inner,
        })
    }

    pub(crate) fn primary_repo_name(&self) -> &str {
        &self.primary_repo_name
    }

    pub(crate) fn primary_repo_id(&self) -> RepoId {
        self.primary_repo_id
    }

    pub(crate) fn lease_primary(&self) -> Result<RepoAuthorityLease, LocalAuthorityError> {
        self.lease(self.primary_repo_id)
    }

    pub(crate) fn lease(&self, repo_id: RepoId) -> Result<RepoAuthorityLease, LocalAuthorityError> {
        lease_from_inner(&self.inner, repo_id)
    }

    pub(crate) fn admit_existing(
        &self,
        repo_id: RepoId,
    ) -> Result<RepoAuthorityLease, LocalAuthorityError> {
        admit_existing_from_inner(&self.inner, repo_id)
    }

    #[cfg(test)]
    pub(crate) fn admit_existing_with_hook_for_test(
        &self,
        repo_id: RepoId,
        before_open: impl FnOnce(),
    ) -> Result<RepoAuthorityLease, LocalAuthorityError> {
        admission::admit_existing_with_hook_for_test(&self.inner, repo_id, before_open)
    }

    #[cfg(test)]
    pub(crate) fn snapshot_for_test(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<RepoAuthoritySlotSnapshot>, LocalAuthorityError> {
        let slots = self
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        Ok(slots.get(&repo_id).map(|slot| match slot {
            RepoAuthoritySlot::Opening { .. } => RepoAuthoritySlotSnapshot::Opening,
            RepoAuthoritySlot::Preparing { generation, .. } => {
                RepoAuthoritySlotSnapshot::Preparing {
                    generation: *generation,
                }
            }
            RepoAuthoritySlot::RepairRequired { generation } => {
                RepoAuthoritySlotSnapshot::RepairRequired {
                    generation: *generation,
                }
            }
            RepoAuthoritySlot::Active { generation, .. } => RepoAuthoritySlotSnapshot::Active {
                generation: *generation,
            },
            RepoAuthoritySlot::Quiescing { generation, .. } => {
                RepoAuthoritySlotSnapshot::Quiescing {
                    generation: *generation,
                }
            }
            RepoAuthoritySlot::CommittedCleanup { generation, .. } => {
                RepoAuthoritySlotSnapshot::CommittedCleanup {
                    generation: *generation,
                }
            }
            RepoAuthoritySlot::Retired { prior_generation } => RepoAuthoritySlotSnapshot::Retired {
                prior_generation: *prior_generation,
            },
        }))
    }
}

#[cfg(test)]
mod tests;
