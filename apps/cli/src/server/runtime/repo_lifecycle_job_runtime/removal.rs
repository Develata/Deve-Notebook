//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 07_network#repo-control-wire-contract
//!
//! Durable Prepare/Execute identities for ownership-aware local removal.

use super::model::RepoLifecycleJobCompletion;
use super::model::{JobFuture, RepoLifecycleJobError};
use super::store::removal::{RemovalCleanupDisposition, RemovalCleanupStep, RemovalExecutionState};
use deve_core::ledger::{
    HostRepoAliasRemovalPlan, ProjectionLocatorRemovalPlan, RepoAuthorityRemovalSnapshot,
    RepoCatalogMembershipRecord,
};
use deve_core::models::RepoId;
use deve_core::protocol::{
    LocalRepoRemovalPreview, OpaqueFallbackBinding, RemovalConfirmationToken,
};
use deve_core::remote_import::RemoteImportRepoRemovalSnapshot;
use deve_core::utils::fs::{HostPathIdentity, HostPathKind};
use deve_core::utils::notegit::NotegitRemovalPlan;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "issuer", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RepoRemovalRepairIssuerBinding {
    LocalCliProxy {
        principal_digest: String,
        runtime_incarnation: Uuid,
    },
    OfflineReceiptAuthority {
        authority_root: HostPathIdentity,
        lifecycle_lock: HostPathIdentity,
    },
}

impl RepoRemovalRepairIssuerBinding {
    pub(super) fn validate(&self) -> Result<(), RepoLifecycleJobError> {
        match self {
            Self::LocalCliProxy {
                principal_digest,
                runtime_incarnation,
            } if is_sha256_hex(principal_digest) && !runtime_incarnation.is_nil() => Ok(()),
            Self::OfflineReceiptAuthority {
                authority_root,
                lifecycle_lock,
            } if authority_root.kind() == HostPathKind::Directory
                && lifecycle_lock.kind() == HostPathKind::RegularFile
                && authority_root.revalidate().unwrap_or(false)
                && lifecycle_lock.revalidate().unwrap_or(false)
                && lifecycle_lock.path().starts_with(authority_root.path()) =>
            {
                Ok(())
            }
            _ => Err(RepoLifecycleJobError::InvalidRequest),
        }
    }
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
    /// Source adapters that carry an explicit target bind it here. The WS v5
    /// shape intentionally relies on its already-bound preparation identity.
    pub(crate) expected_repo_id: Option<RepoId>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemovalRepairToken(String);

impl RemovalRepairToken {
    pub(crate) fn from_backend(value: String) -> Option<Self> {
        (value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then_some(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepoRemovalRepairTarget {
    RemoteImportArtifacts,
    ProcessRuntimeSlots,
    NotegitTree,
    LocalAuthorityDatabase,
    ProjectionLocator,
    HostAlias,
    CatalogTombstone,
    AuthorityRetirement,
    TerminalReceipt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepoRemovalRepairTruth {
    Exact,
    AlreadyAbsent,
    Changed,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepoRemovalRepairItem {
    pub(crate) target: RepoRemovalRepairTarget,
    pub(crate) truth: RepoRemovalRepairTruth,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepoRemovalRepairInspection {
    pub(crate) request_id: Uuid,
    pub(crate) repo_id: RepoId,
    pub(crate) remaining: Vec<RepoRemovalRepairItem>,
    pub(crate) apply_allowed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoRemovalRepairPrepared {
    pub(crate) inspection: RepoRemovalRepairInspection,
    pub(crate) token: Option<RemovalRepairToken>,
    pub(crate) expires_at_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoRemovalRepairApplyIntent {
    pub(crate) request_id: Uuid,
    pub(crate) token: RemovalRepairToken,
    pub(crate) issuer: RepoRemovalRepairIssuerBinding,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepoRemovalFallbackSnapshot {
    pub(crate) repo_id: RepoId,
    pub(crate) membership_revision: u64,
    pub(crate) authority_generation: u64,
    pub(crate) watcher_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepoRemovalManifest {
    pub(crate) repo_id: RepoId,
    pub(crate) catalog: RepoCatalogMembershipRecord,
    pub(crate) authority: RepoAuthorityRemovalSnapshot,
    pub(crate) locator: ProjectionLocatorRemovalPlan,
    pub(crate) notegit: NotegitRemovalPlan,
    pub(crate) alias: HostRepoAliasRemovalPlan,
    pub(crate) watcher_generation: u64,
    pub(crate) remote_import: RemoteImportRepoRemovalSnapshot,
    pub(crate) fallback: Option<RepoRemovalFallbackSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoRemovalPreparation {
    pub(super) manifest: Option<RepoRemovalManifest>,
    pub(super) preview: LocalRepoRemovalPreview,
}

pub(crate) struct RepoRemovalExecution {
    pub(crate) preparation_id: Uuid,
    pub(crate) execute_request_id: Uuid,
    pub(crate) manifest_digest: String,
    pub(crate) manifest: RepoRemovalManifest,
    pub(crate) state: RemovalExecutionState,
    pub(crate) progress: RepoRemovalProgress,
}

#[derive(Clone)]
pub(crate) struct RepoRemovalProgress {
    preparation_id: Uuid,
    execute_request_id: Uuid,
    sender: mpsc::Sender<RemovalProgressCommand>,
}

pub(super) struct RemovalProgressCommand {
    pub(super) preparation_id: Uuid,
    pub(super) execute_request_id: Uuid,
    pub(super) update: RemovalProgressUpdate,
    pub(super) reply: oneshot::Sender<Result<RemovalExecutionState, RepoLifecycleJobError>>,
}

pub(super) enum RemovalProgressUpdate {
    SealRemoteImport(Box<deve_core::remote_import::RemoteImportRepoRemovalPlan>),
    CutAttempted,
    CutObserved(RepoCatalogMembershipRecord),
    CutNotCommitted,
    RemoteImportCheckpoint(Box<deve_core::remote_import::RemoteImportRepoRemovalCheckpoint>),
    NotegitCheckpoint(deve_core::utils::notegit::NotegitRemovalCheckpoint),
    AuthorityCheckpoint(Box<deve_core::ledger::RepoAuthorityDatabaseCheckpoint>),
    CleanupStep {
        step: RemovalCleanupStep,
        disposition: RemovalCleanupDisposition,
    },
    CleanupComplete,
    TombstoneRetired,
    TerminalCandidate(Box<RepoLifecycleJobCompletion>),
    TerminalComplete,
}

impl RepoRemovalProgress {
    pub(super) fn new(
        preparation_id: Uuid,
        execute_request_id: Uuid,
        sender: mpsc::Sender<RemovalProgressCommand>,
    ) -> Self {
        Self {
            preparation_id,
            execute_request_id,
            sender,
        }
    }

    async fn record(
        &self,
        update: RemovalProgressUpdate,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(RemovalProgressCommand {
                preparation_id: self.preparation_id,
                execute_request_id: self.execute_request_id,
                update,
                reply,
            })
            .await
            .map_err(|_| RepoLifecycleJobError::Coordination("removal progress owner stopped"))?;
        response
            .await
            .map_err(|_| RepoLifecycleJobError::Coordination("removal progress reply dropped"))?
    }

    pub(crate) async fn seal_remote_import(
        &self,
        plan: deve_core::remote_import::RemoteImportRepoRemovalPlan,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::SealRemoteImport(Box::new(plan)))
            .await
    }

    pub(crate) async fn cut_attempted(
        &self,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::CutAttempted).await
    }

    pub(crate) async fn cut_observed(
        &self,
        tombstone: RepoCatalogMembershipRecord,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::CutObserved(tombstone))
            .await
    }

    pub(crate) async fn cut_not_committed(
        &self,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::CutNotCommitted).await
    }

    pub(crate) async fn cleanup_step(
        &self,
        step: RemovalCleanupStep,
        disposition: RemovalCleanupDisposition,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::CleanupStep { step, disposition })
            .await
    }

    pub(crate) async fn remote_import_checkpoint(
        &self,
        checkpoint: deve_core::remote_import::RemoteImportRepoRemovalCheckpoint,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::RemoteImportCheckpoint(Box::new(
            checkpoint,
        )))
        .await
    }

    pub(crate) async fn notegit_checkpoint(
        &self,
        checkpoint: deve_core::utils::notegit::NotegitRemovalCheckpoint,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::NotegitCheckpoint(checkpoint))
            .await
    }

    pub(crate) async fn authority_checkpoint(
        &self,
        checkpoint: deve_core::ledger::RepoAuthorityDatabaseCheckpoint,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::AuthorityCheckpoint(Box::new(
            checkpoint,
        )))
        .await
    }

    pub(crate) async fn cleanup_complete(
        &self,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::CleanupComplete).await
    }

    pub(crate) async fn tombstone_retired(
        &self,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::TombstoneRetired).await
    }

    pub(crate) async fn terminal_candidate(
        &self,
        completion: RepoLifecycleJobCompletion,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::TerminalCandidate(Box::new(
            completion,
        )))
        .await
    }

    pub(crate) async fn terminal_complete(
        &self,
    ) -> Result<RemovalExecutionState, RepoLifecycleJobError> {
        self.record(RemovalProgressUpdate::TerminalComplete).await
    }
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
