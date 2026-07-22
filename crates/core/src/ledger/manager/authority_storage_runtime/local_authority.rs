//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#local-repo-removal-contract
//!
//! Process-local owner of every local Redb authority handle.

mod admission;
mod checkpoint;
mod creation;
mod drainage;
mod inspection;
mod prepared;
mod reservation;
mod resource;
mod retirement;

use crate::models::RepoId;
use crate::utils::fs::{HostPathIdentity, HostPathKind, HostQuarantinePlan};
use admission::PrimaryRepoBinding;
use admission::{admit_existing_from_inner, lease_from_inner};
pub use checkpoint::{
    RepoAuthorityDatabaseCheckpoint, RepoAuthorityRemovalSnapshot, RepoAuthorityRetirementProof,
};
use redb::Database;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};
use thiserror::Error;
use uuid::Uuid;

pub use prepared::PreparedRepoAuthority;
pub(crate) use prepared::{PreparedAuthorityIdentity, PreparedRepoAuthorityOrigin};

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
    #[error("no local repository is mounted in the current host runtime")]
    NoLocalRepo,
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
    #[error("local authority cleanup identity changed for RepoId {0}")]
    CleanupIdentityChanged(RepoId),
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
    authority_lock: Arc<File>,
    lock_path: PathBuf,
    db_path: PathBuf,
}

#[allow(dead_code)] // R3/R4 consumes terminal generations outside test builds.
enum RepoAuthoritySlot {
    Opening {
        reservation_id: Uuid,
    },
    Reopening {
        reservation_id: Uuid,
        generation: u64,
        expected_lock_identity: HostPathIdentity,
        removed_database_identity: HostPathIdentity,
        authority_lock: Option<Arc<File>>,
        resources: Option<Arc<RepoAuthorityResources>>,
    },
    Preparing {
        reservation_id: Uuid,
        generation: u64,
        resources: Arc<RepoAuthorityResources>,
    },
    ReopeningPrepared {
        reservation_id: Uuid,
        generation: u64,
        expected_lock_identity: HostPathIdentity,
        removed_database_identity: HostPathIdentity,
        resources: Arc<RepoAuthorityResources>,
    },
    ReopeningRepairRequired {
        generation: u64,
        expected_lock_identity: HostPathIdentity,
        removed_database_identity: HostPathIdentity,
        authority_lock: Option<Arc<File>>,
        resources: Option<Arc<RepoAuthorityResources>>,
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
        authority_lock: Arc<File>,
        expected_lock_identity: HostPathIdentity,
        removed_database_identity: HostPathIdentity,
        db_path: PathBuf,
        cleanup_capability_issued: bool,
    },
    Retired {
        prior_generation: u64,
        expected_lock_identity: HostPathIdentity,
        removed_database_identity: HostPathIdentity,
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
    primary_repo: OnceLock<PrimaryRepoBinding>,
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

/// Exclusive pre-commit capability for a repo authority cut.
///
/// Dropping this value rolls the slot back to `Active`. Only the removal
/// coordinator may cross the durable commit cut with `into_committed_cleanup`.
#[allow(dead_code)] // Consumed by the approved R3/R4 removal coordinator.
pub struct RepoAuthorityQuiesceGuard {
    inner: Arc<LocalAuthorityInner>,
    resources: Option<Arc<RepoAuthorityResources>>,
    repo_id: RepoId,
    generation: u64,
    settled: bool,
}

/// Post-commit cleanup capability. The Redb handle is already closed while the
/// cross-process owner lock remains held.
#[allow(dead_code)] // Consumed by the approved R4 removal coordinator.
pub struct RepoAuthorityCleanupGuard {
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

    pub fn removal_snapshot(&self) -> Result<RepoAuthorityRemovalSnapshot, LocalAuthorityError> {
        let runtime = self.runtime.upgrade().ok_or_else(|| {
            LocalAuthorityError::Invariant(
                "local authority runtime disappeared while a lease remained live".to_string(),
            )
        })?;
        {
            let slots = runtime
                .slots
                .lock()
                .map_err(|_| LocalAuthorityError::Poisoned)?;
            match slots.get(&self.repo_id) {
                Some(RepoAuthoritySlot::Active {
                    generation,
                    resources,
                    leases: 1,
                }) if *generation == self.generation && Arc::ptr_eq(resources, &self.resources) => {
                }
                Some(RepoAuthoritySlot::Active { .. }) => {
                    return Err(LocalAuthorityError::Busy(self.repo_id));
                }
                Some(RepoAuthoritySlot::Quiescing { .. }) => {
                    return Err(LocalAuthorityError::Quiescing(self.repo_id));
                }
                Some(RepoAuthoritySlot::Retired { .. }) => {
                    return Err(LocalAuthorityError::Retired(self.repo_id));
                }
                _ => {
                    return Err(LocalAuthorityError::Invariant(format!(
                        "RepoId {} removal lease no longer matches its active slot",
                        self.repo_id
                    )));
                }
            }
        }
        resource::validate_resource_identity(&self.resources)?;
        let database =
            HostPathIdentity::capture(&self.resources.db_path, HostPathKind::RegularFile)?;
        let quarantine_id = Uuid::new_v4().simple().to_string();
        let database_parent = self.resources.db_path.parent().ok_or_else(|| {
            LocalAuthorityError::Invariant("local authority database has no parent".to_string())
        })?;
        let database_quarantine = HostQuarantinePlan::same_parent(
            database.clone(),
            database_parent.join(format!(
                ".deve-removing-{quarantine_id}-{}.redb",
                self.repo_id
            )),
        )?;
        Ok(RepoAuthorityRemovalSnapshot {
            repo_id: self.repo_id,
            generation: self.generation,
            database,
            database_quarantine,
            authority_lock: HostPathIdentity::capture(
                &self.resources.lock_path,
                HostPathKind::RegularFile,
            )?,
        })
    }

    pub(crate) fn identity_observation(
        &self,
    ) -> Result<PreparedAuthorityIdentity, LocalAuthorityError> {
        resource::validate_resource_identity(&self.resources)?;
        let database =
            HostPathIdentity::capture(&self.resources.db_path, HostPathKind::RegularFile)?;
        let authority_lock =
            HostPathIdentity::capture(&self.resources.lock_path, HostPathKind::RegularFile)?;
        crate::utils::fs::ensure_open_file_matches_identity(
            &self.resources.db_witness,
            &database,
            "local authority database identity",
        )?;
        crate::utils::fs::ensure_open_file_matches_identity(
            &self.resources.authority_lock,
            &authority_lock,
            "local authority lock identity",
        )?;
        Ok(PreparedAuthorityIdentity {
            database,
            authority_lock,
        })
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
use tests::RepoAuthoritySlotSnapshot;

impl LocalAuthorityRuntime {
    pub(crate) fn empty(ledger_dir: &Path) -> Self {
        Self {
            primary_repo: OnceLock::new(),
            inner: Arc::new(LocalAuthorityInner {
                ledger_dir: ledger_dir.to_path_buf(),
                slots: Mutex::new(HashMap::new()),
                lease_released: Condvar::new(),
            }),
        }
    }

    pub(crate) fn prepare_new_initialized<F>(
        ledger_dir: &Path,
        primary_repo_id: RepoId,
        initialize: F,
    ) -> Result<(Self, PreparedRepoAuthority), LocalAuthorityError>
    where
        F: FnOnce(&Database) -> Result<(), LocalAuthorityError>,
    {
        let runtime = Self {
            primary_repo: OnceLock::from(PrimaryRepoBinding::new(primary_repo_id)),
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
            primary_repo: OnceLock::from(PrimaryRepoBinding::new(primary_repo_id)),
            inner,
        })
    }

    pub(crate) fn primary_repo_name(&self) -> Option<&str> {
        self.primary_repo
            .get()
            .map(|binding| binding.execution_name.as_str())
    }

    pub(crate) fn primary_repo_id(&self) -> Option<RepoId> {
        self.primary_repo.get().map(|binding| binding.repo_id)
    }

    pub(crate) fn lease_primary(&self) -> Result<RepoAuthorityLease, LocalAuthorityError> {
        self.lease(
            self.primary_repo
                .get()
                .map(|binding| binding.repo_id)
                .ok_or(LocalAuthorityError::NoLocalRepo)?,
        )
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
            RepoAuthoritySlot::Reopening { generation, .. } => {
                RepoAuthoritySlotSnapshot::Reopening {
                    generation: *generation,
                }
            }
            RepoAuthoritySlot::Preparing { generation, .. } => {
                RepoAuthoritySlotSnapshot::Preparing {
                    generation: *generation,
                }
            }
            RepoAuthoritySlot::ReopeningPrepared { generation, .. } => {
                RepoAuthoritySlotSnapshot::ReopeningPrepared {
                    generation: *generation,
                }
            }
            RepoAuthoritySlot::ReopeningRepairRequired { generation, .. } => {
                RepoAuthoritySlotSnapshot::RepairRequired {
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
            RepoAuthoritySlot::Retired {
                prior_generation, ..
            } => RepoAuthoritySlotSnapshot::Retired {
                prior_generation: *prior_generation,
            },
        }))
    }
}

#[cfg(test)]
mod tests;
