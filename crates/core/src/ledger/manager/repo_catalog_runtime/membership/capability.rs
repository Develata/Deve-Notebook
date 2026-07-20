//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Capability objects handed out by the catalog membership runtime: opaque
//! membership tokens, the unique cut authority, and the bounded cut permits it
//! mints. The slot machine that guards them stays in the parent module.

use super::CatalogMembershipRuntime;
use crate::models::RepoId;
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CatalogMembershipGeneration(pub(super) u64);

impl CatalogMembershipGeneration {
    pub(super) const INITIAL: Self = Self(1);

    pub const fn get(self) -> u64 {
        self.0
    }

    pub(super) fn next(self, repo_id: RepoId) -> Result<Self, CatalogMembershipError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(CatalogMembershipError::GenerationExhausted(repo_id))
    }
}

/// Opaque process-local evidence that one repo was a normal catalog member at
/// one exact per-repo generation. The token owns no mutation authority.
#[derive(Clone, PartialEq, Eq)]
pub struct CatalogMembershipToken {
    pub(super) runtime_instance: Uuid,
    pub(super) repo_id: RepoId,
    pub(super) generation: CatalogMembershipGeneration,
}

/// Unforgeable evidence that the host owns the short `Catalog -> Repo` cut
/// lane for one exact process runtime and RepoId.
///
/// B1 intentionally exposes no public constructor. C1' transfers the sole
/// owner capability into `RepoMutationPublicationGate`, which will mint this
/// borrowed proof only while both ordered permits are held.
pub struct RepoCatalogCutPermit {
    pub(super) runtime_instance: Uuid,
    pub(super) authority_instance: Uuid,
    pub(super) repo_id: RepoId,
}

/// Unique host capability that may mint bounded catalog-cut permits.
///
/// The capability is deliberately non-`Clone`. `RepoManager` only lends it
/// through an exact claim operation, and dropping the owner invalidates every
/// permit minted by that owner before a replacement may be claimed.
pub struct RepoCatalogCutAuthority {
    pub(super) runtime: CatalogMembershipRuntime,
    pub(super) authority_instance: Uuid,
}

impl fmt::Debug for RepoCatalogCutPermit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepoCatalogCutPermit")
            .field("repo_id", &self.repo_id)
            .finish_non_exhaustive()
    }
}

impl CatalogMembershipToken {
    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub const fn generation(&self) -> CatalogMembershipGeneration {
        self.generation
    }
}

impl fmt::Debug for CatalogMembershipToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CatalogMembershipToken")
            .field("repo_id", &self.repo_id)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogMembershipError {
    #[error("catalog membership runtime has not been seeded")]
    NotSeeded,
    #[error("catalog membership bootstrap seed differs from current runtime membership")]
    SeedDrift,
    #[error("catalog membership seed contains duplicate RepoId {0}")]
    DuplicateSeed(RepoId),
    #[error("repo {0} is not a current catalog member")]
    NotMember(RepoId),
    #[error("repo {0} is already a current catalog member")]
    AlreadyMember(RepoId),
    #[error("catalog membership token for repo {0} belongs to another runtime instance")]
    RuntimeMismatch(RepoId),
    #[error(
        "catalog membership token for repo {repo_id} is stale: expected generation {expected:?}, current generation {current:?}"
    )]
    Stale {
        repo_id: RepoId,
        expected: CatalogMembershipGeneration,
        current: CatalogMembershipGeneration,
    },
    #[error("catalog membership generation exhausted for repo {0}")]
    GenerationExhausted(RepoId),
    #[error("catalog membership runtime lock is poisoned")]
    Poisoned,
    #[error("catalog membership ledger identity is invalid: {0}")]
    InvalidLedgerIdentity(String),
    #[error("repo catalog cut permit does not belong to current runtime/repo {0}")]
    CutPermitMismatch(RepoId),
    #[error("repo catalog cut authority is already claimed")]
    CutAuthorityAlreadyClaimed,
}

impl RepoCatalogCutAuthority {
    /// Mints proof for one exact repo. The caller must keep this proof inside
    /// the already-held ordered `Catalog -> Repo` critical section.
    pub fn permit(&self, repo_id: RepoId) -> Result<RepoCatalogCutPermit, CatalogMembershipError> {
        let owner = self
            .runtime
            .inner
            .cut_authority
            .lock()
            .map_err(|_| CatalogMembershipError::Poisoned)?;
        if owner.as_ref() != Some(&self.authority_instance) {
            return Err(CatalogMembershipError::CutPermitMismatch(repo_id));
        }
        Ok(RepoCatalogCutPermit {
            runtime_instance: self.runtime.inner.runtime_instance,
            authority_instance: self.authority_instance,
            repo_id,
        })
    }
}

impl std::fmt::Debug for RepoCatalogCutAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepoCatalogCutAuthority")
            .finish_non_exhaustive()
    }
}

impl Drop for RepoCatalogCutAuthority {
    fn drop(&mut self) {
        let Ok(mut owner) = self.runtime.inner.cut_authority.lock() else {
            return;
        };
        if owner.as_ref() == Some(&self.authority_instance) {
            *owner = None;
        }
    }
}
