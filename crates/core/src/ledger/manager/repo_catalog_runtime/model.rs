//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/index#repo-runtime-layout

use super::store::RepoCatalogStore;
use super::{CatalogMembershipToken, RepoCatalogError};
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(super) const CATALOG_RECORD_FORMAT: &str = "deve.host-repo-membership";
pub(super) const CATALOG_RECORD_VERSION: u32 = 1;
pub(super) const CATALOG_RECORD_MAX_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoCatalogMembershipState {
    Normal,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoCatalogMembershipRecord {
    pub(super) format: String,
    pub(super) version: u32,
    pub(super) repo_id: RepoId,
    pub(super) state: RepoCatalogMembershipState,
    pub(super) membership_revision: u64,
    pub(super) prepared_identity_digest: String,
    pub(super) lifecycle_request_id: Uuid,
}

impl RepoCatalogMembershipRecord {
    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub const fn state(&self) -> RepoCatalogMembershipState {
        self.state
    }

    pub const fn membership_revision(&self) -> u64 {
        self.membership_revision
    }

    pub const fn lifecycle_request_id(&self) -> Uuid {
        self.lifecycle_request_id
    }

    pub fn prepared_identity_digest(&self) -> &str {
        &self.prepared_identity_digest
    }

    pub(super) fn normal(
        repo_id: RepoId,
        identity: PreparedRepoIdentity,
        lifecycle_request_id: Uuid,
    ) -> Self {
        Self {
            format: CATALOG_RECORD_FORMAT.to_string(),
            version: CATALOG_RECORD_VERSION,
            repo_id,
            state: RepoCatalogMembershipState::Normal,
            membership_revision: 1,
            prepared_identity_digest: identity.to_hex(),
            lifecycle_request_id,
        }
    }

    pub(super) fn removed(
        normal: &Self,
        lifecycle_request_id: Uuid,
    ) -> Result<Self, RepoCatalogError> {
        let revision = normal.membership_revision.checked_add(1).ok_or(
            RepoCatalogError::MembershipRevisionExhausted(normal.repo_id),
        )?;
        Ok(Self {
            format: CATALOG_RECORD_FORMAT.to_string(),
            version: CATALOG_RECORD_VERSION,
            repo_id: normal.repo_id,
            state: RepoCatalogMembershipState::Removed,
            membership_revision: revision,
            prepared_identity_digest: normal.prepared_identity_digest.clone(),
            lifecycle_request_id,
        })
    }

    pub(super) fn validate(&self, expected_repo_id: RepoId) -> Result<(), RepoCatalogError> {
        if self.format != CATALOG_RECORD_FORMAT {
            return Err(RepoCatalogError::InvalidRecord(format!(
                "unsupported format {:?}",
                self.format
            )));
        }
        if self.version != CATALOG_RECORD_VERSION {
            return Err(RepoCatalogError::InvalidRecord(format!(
                "unsupported version {}",
                self.version
            )));
        }
        if self.repo_id != expected_repo_id {
            return Err(RepoCatalogError::RecordIdentityMismatch {
                expected: expected_repo_id,
                actual: self.repo_id,
            });
        }
        if self.membership_revision == 0 {
            return Err(RepoCatalogError::InvalidRecord(
                "membership_revision must be positive".to_string(),
            ));
        }
        if self.lifecycle_request_id.is_nil() {
            return Err(RepoCatalogError::InvalidRecord(
                "lifecycle_request_id must not be nil".to_string(),
            ));
        }
        PreparedRepoIdentity::from_hex(&self.prepared_identity_digest)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedRepoIdentity([u8; 32]);

impl PreparedRepoIdentity {
    pub(super) fn from_manifest_bytes(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    pub(super) fn from_hex(value: &str) -> Result<Self, RepoCatalogError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(RepoCatalogError::InvalidRecord(
                "prepared_identity_digest must be 64 lowercase hex characters".to_string(),
            ));
        }
        let mut bytes = [0_u8; 32];
        hex::decode_to_slice(value, &mut bytes).map_err(|error| {
            RepoCatalogError::InvalidRecord(format!(
                "prepared_identity_digest is not valid hex: {error}"
            ))
        })?;
        Ok(Self(bytes))
    }

    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

#[derive(Debug, Clone)]
pub struct PreparedRepoCreation {
    pub(super) repo_id: RepoId,
    pub(super) lifecycle_request_id: Uuid,
    pub(super) prepared_identity: PreparedRepoIdentity,
}

impl PreparedRepoCreation {
    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub const fn lifecycle_request_id(&self) -> Uuid {
        self.lifecycle_request_id
    }

    pub const fn prepared_identity(&self) -> PreparedRepoIdentity {
        self.prepared_identity
    }
}

#[derive(Debug, Clone)]
pub struct PreparedRepoRemoval {
    pub(super) normal_record: RepoCatalogMembershipRecord,
    pub(super) membership: CatalogMembershipToken,
    pub(super) prepared_identity: PreparedRepoIdentity,
    pub(super) lifecycle_request_id: Uuid,
}

pub struct RevalidatedRepoCreation {
    pub(super) repo_id: RepoId,
    pub(super) lifecycle_request_id: Uuid,
    pub(super) prepared_identity: PreparedRepoIdentity,
    pub(super) store: RepoCatalogStore,
}

impl std::fmt::Debug for RevalidatedRepoCreation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RevalidatedRepoCreation")
            .field("repo_id", &self.repo_id)
            .field("lifecycle_request_id", &self.lifecycle_request_id)
            .finish_non_exhaustive()
    }
}

pub struct RevalidatedRepoRemoval {
    pub(super) repo_id: RepoId,
    pub(super) lifecycle_request_id: Uuid,
    pub(super) prepared_identity: PreparedRepoIdentity,
    pub(super) store: RepoCatalogStore,
}

impl std::fmt::Debug for RevalidatedRepoRemoval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RevalidatedRepoRemoval")
            .field("repo_id", &self.repo_id)
            .field("lifecycle_request_id", &self.lifecycle_request_id)
            .finish_non_exhaustive()
    }
}

impl PreparedRepoRemoval {
    pub const fn repo_id(&self) -> RepoId {
        self.normal_record.repo_id
    }

    pub const fn lifecycle_request_id(&self) -> Uuid {
        self.lifecycle_request_id
    }

    pub fn membership(&self) -> &CatalogMembershipToken {
        &self.membership
    }

    pub const fn prepared_identity(&self) -> PreparedRepoIdentity {
        self.prepared_identity
    }
}

#[derive(Debug, Clone)]
pub struct RepoCatalogCreationCommit {
    record: RepoCatalogMembershipRecord,
    membership: CatalogMembershipToken,
}

impl RepoCatalogCreationCommit {
    pub(super) fn new(
        record: RepoCatalogMembershipRecord,
        membership: CatalogMembershipToken,
    ) -> Self {
        Self { record, membership }
    }

    pub fn record(&self) -> &RepoCatalogMembershipRecord {
        &self.record
    }

    pub fn membership(&self) -> &CatalogMembershipToken {
        &self.membership
    }
}

#[derive(Debug, Clone)]
pub struct RepoCatalogRemovalCommit {
    record: RepoCatalogMembershipRecord,
    revoked_membership: CatalogMembershipToken,
}

impl RepoCatalogRemovalCommit {
    pub(super) fn new(
        record: RepoCatalogMembershipRecord,
        revoked_membership: CatalogMembershipToken,
    ) -> Self {
        Self {
            record,
            revoked_membership,
        }
    }

    pub fn record(&self) -> &RepoCatalogMembershipRecord {
        &self.record
    }

    pub fn revoked_membership(&self) -> &CatalogMembershipToken {
        &self.revoked_membership
    }
}
