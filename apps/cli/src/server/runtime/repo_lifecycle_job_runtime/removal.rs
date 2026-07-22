//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 07_network#repo-control-wire-contract
//!
//! Durable Prepare/Execute identities for ownership-aware local removal.

use super::model::{JobFuture, RepoLifecycleJobError};
use deve_core::ledger::{
    ProjectionLocatorRecord, RepoAuthorityRemovalSnapshot, RepoCatalogMembershipRecord,
};
use deve_core::models::RepoId;
use deve_core::protocol::{
    LocalRepoRemovalPreview, OpaqueFallbackBinding, RemovalConfirmationToken,
};
use deve_core::remote_import::RemoteImportRepoRemovalSnapshot;
use deve_core::utils::fs::{HostPathIdentity, HostPathKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "issuer", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RepoRemovalIssuerBinding {
    Web {
        principal_digest: String,
        connection_epoch: u64,
    },
    LocalCliProxy {
        principal_digest: String,
    },
    OfflineAuthority {
        authority_root: HostPathIdentity,
        authority_lock: HostPathIdentity,
    },
}

impl RepoRemovalIssuerBinding {
    pub(super) fn validate(&self) -> Result<(), RepoLifecycleJobError> {
        match self {
            Self::Web {
                principal_digest,
                connection_epoch,
            } => {
                if *connection_epoch == 0 || !is_sha256_hex(principal_digest) {
                    return Err(RepoLifecycleJobError::InvalidRequest);
                }
            }
            Self::LocalCliProxy { principal_digest } => {
                if !is_sha256_hex(principal_digest) {
                    return Err(RepoLifecycleJobError::InvalidRequest);
                }
            }
            Self::OfflineAuthority {
                authority_root,
                authority_lock,
            } => {
                if authority_root.kind() != HostPathKind::Directory
                    || authority_lock.kind() != HostPathKind::RegularFile
                    || !authority_root.revalidate().unwrap_or(false)
                    || !authority_lock.revalidate().unwrap_or(false)
                {
                    return Err(RepoLifecycleJobError::InvalidRequest);
                }
            }
        }
        Ok(())
    }

    pub(super) const fn is_runtime_bound(&self) -> bool {
        matches!(self, Self::Web { .. } | Self::LocalCliProxy { .. })
    }

    pub(super) fn binds_manifest(&self, manifest: Option<&RepoRemovalManifest>) -> bool {
        match self {
            Self::Web { .. } | Self::LocalCliProxy { .. } => true,
            Self::OfflineAuthority {
                authority_root,
                authority_lock,
            } => manifest.is_none_or(|manifest| {
                authority_lock == manifest.authority.authority_lock()
                    && manifest
                        .authority
                        .database()
                        .path()
                        .starts_with(authority_root.path())
                    && authority_lock.path().starts_with(authority_root.path())
            }),
        }
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoRemovalPrepareIntent {
    pub(crate) request_id: Uuid,
    pub(crate) repo_id: RepoId,
    pub(crate) scope_nonce: u64,
    pub(crate) fallback_repo_id: Option<RepoId>,
    pub(crate) issuer: RepoRemovalIssuerBinding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoRemovalExecuteIntent {
    pub(crate) request_id: Uuid,
    pub(crate) preparation_id: Uuid,
    pub(crate) confirmation_token: RemovalConfirmationToken,
    pub(crate) fallback_binding: Option<OpaqueFallbackBinding>,
    pub(crate) scope_nonce: u64,
    pub(crate) switch_nonce: u64,
    pub(crate) issuer: RepoRemovalIssuerBinding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoRemovalPrepared {
    pub(crate) request_id: Uuid,
    pub(crate) preparation_id: Uuid,
    pub(crate) repo_id: RepoId,
    pub(crate) preview: LocalRepoRemovalPreview,
    pub(crate) confirmation_token: Option<RemovalConfirmationToken>,
    pub(crate) fallback_binding: Option<OpaqueFallbackBinding>,
    pub(crate) expires_at_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RepoRemovalFallbackSnapshot {
    pub(super) repo_id: RepoId,
    pub(super) membership_revision: u64,
    pub(super) authority_generation: u64,
    pub(super) watcher_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RepoRemovalManifest {
    pub(super) repo_id: RepoId,
    pub(super) catalog: RepoCatalogMembershipRecord,
    pub(super) authority: RepoAuthorityRemovalSnapshot,
    pub(super) locator: ProjectionLocatorRecord,
    pub(super) workspace_root: HostPathIdentity,
    pub(super) notegit_root: HostPathIdentity,
    pub(super) identity_marker: HostPathIdentity,
    pub(super) identity_marker_digest: String,
    pub(super) alias_revision: u64,
    pub(super) watcher_generation: u64,
    pub(super) remote_import: RemoteImportRepoRemovalSnapshot,
    pub(super) fallback: Option<RepoRemovalFallbackSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoRemovalPreparation {
    pub(super) manifest: Option<RepoRemovalManifest>,
    pub(super) preview: LocalRepoRemovalPreview,
}

pub(crate) trait RepoRemovalPlanner: Send + Sync + 'static {
    fn prepare_removal(
        &self,
        _intent: RepoRemovalPrepareIntent,
    ) -> JobFuture<Result<RepoRemovalPreparation, RepoLifecycleJobError>> {
        Box::pin(async { Err(RepoLifecycleJobError::RemovalBlocked) })
    }

    fn revalidate_removal(
        &self,
        _manifest: RepoRemovalManifest,
    ) -> JobFuture<Result<(), RepoLifecycleJobError>> {
        Box::pin(async { Err(RepoLifecycleJobError::ConfirmationStale) })
    }
}
