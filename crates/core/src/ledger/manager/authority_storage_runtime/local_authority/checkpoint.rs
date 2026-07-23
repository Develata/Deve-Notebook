//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#local-repo-removal-contract
//!
//! Opaque durable progress for canonical Redb quarantine cleanup.

use crate::models::RepoId;
use crate::utils::fs::{HostPathIdentity, HostPathState, HostQuarantineCut, HostQuarantinePlan};
use serde::{Deserialize, Serialize};
use std::fs::File;

/// Short-lived proof that terminal removal finalization re-acquired the
/// persistent owner lock after authority retirement and revalidated absence.
pub struct RepoAuthorityRetirementProof {
    pub(super) _authority_lock: File,
    pub(super) repo_id: RepoId,
    pub(super) generation: u64,
}

impl RepoAuthorityRetirementProof {
    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoAuthorityRemovalSnapshot {
    pub(super) repo_id: RepoId,
    pub(super) generation: u64,
    pub(super) database: HostPathIdentity,
    pub(super) database_quarantine: HostQuarantinePlan,
    pub(super) authority_lock: HostPathIdentity,
}

impl RepoAuthorityRemovalSnapshot {
    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn database(&self) -> &HostPathIdentity {
        &self.database
    }

    pub fn authority_lock(&self) -> &HostPathIdentity {
        &self.authority_lock
    }

    pub fn revalidate(&self) -> std::io::Result<bool> {
        Ok(self.database_quarantine.revalidate_prepared()? && self.authority_lock.revalidate()?)
    }

    pub fn initial_database_checkpoint(&self) -> RepoAuthorityDatabaseCheckpoint {
        RepoAuthorityDatabaseCheckpoint {
            state: RepoAuthorityDatabaseCheckpointState::Prepared,
        }
    }

    pub fn verify_database_cleanup_complete(
        &self,
        checkpoint: &RepoAuthorityDatabaseCheckpoint,
    ) -> std::io::Result<bool> {
        let RepoAuthorityDatabaseCheckpointState::DatabaseDeleted { database } = &checkpoint.state
        else {
            return Ok(false);
        };
        Ok(database.belongs_to(&self.database_quarantine) && database.is_deleted()?)
    }

    /// Read-only repair admission. A replacement DB or lock is never rebound,
    /// even when its pathname or embedded RepoId text matches.
    pub fn repair_retry_is_exact(
        &self,
        checkpoint: &RepoAuthorityDatabaseCheckpoint,
    ) -> std::io::Result<bool> {
        if self.authority_lock.classify()? != HostPathState::Exact {
            return Ok(false);
        }
        let exact = match &checkpoint.state {
            RepoAuthorityDatabaseCheckpointState::Prepared => {
                match self.database_quarantine.observe_cut() {
                    Ok(Some(database)) => {
                        database.original_path_is_absent()? && database.is_quarantined_exact()?
                    }
                    Ok(None) => self.database_quarantine.revalidate_prepared()?,
                    Err(_) => false,
                }
            }
            RepoAuthorityDatabaseCheckpointState::DatabaseQuarantined { database } => {
                database.belongs_to(&self.database_quarantine)
                    && database.original_path_is_absent()?
                    && database.is_quarantined_exact()?
            }
            RepoAuthorityDatabaseCheckpointState::DatabaseDeleted { database } => {
                database.belongs_to(&self.database_quarantine) && database.is_deleted()?
            }
        };
        Ok(exact)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoAuthorityDatabaseCheckpoint {
    pub(super) state: RepoAuthorityDatabaseCheckpointState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RepoAuthorityDatabaseCheckpointState {
    Prepared,
    DatabaseQuarantined { database: HostQuarantineCut },
    DatabaseDeleted { database: HostQuarantineCut },
}

impl RepoAuthorityDatabaseCheckpoint {
    pub fn is_complete(&self) -> bool {
        matches!(
            self.state,
            RepoAuthorityDatabaseCheckpointState::DatabaseDeleted { .. }
        )
    }
}
