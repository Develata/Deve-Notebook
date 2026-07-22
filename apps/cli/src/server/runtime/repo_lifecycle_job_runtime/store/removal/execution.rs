//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Monotonic durable state for the post-cut removal saga.

use super::super::{LifecycleReceipt, store_invalid};
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleJobCompletion, RepoLifecycleJobError,
};
use deve_core::ledger::{RepoAuthorityDatabaseCheckpoint, RepoCatalogMembershipRecord};
use deve_core::remote_import::{RemoteImportRepoRemovalCheckpoint, RemoteImportRepoRemovalPlan};
use deve_core::utils::notegit::NotegitRemovalCheckpoint;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RemovalCleanupStep {
    RemoteImportArtifacts,
    ProcessRuntimeSlots,
    NotegitTree,
    LocalAuthorityDatabase,
    ProjectionLocator,
    HostAlias,
}

impl RemovalCleanupStep {
    pub(crate) const ORDER: [Self; 6] = [
        Self::RemoteImportArtifacts,
        Self::ProcessRuntimeSlots,
        Self::NotegitTree,
        Self::LocalAuthorityDatabase,
        Self::ProjectionLocator,
        Self::HostAlias,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RemovalCleanupDisposition {
    Deleted,
    AlreadyAbsent,
    Retired,
    Failed,
}

impl RemovalCleanupDisposition {
    pub(crate) const fn is_success(self) -> bool {
        !matches!(self, Self::Failed)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemovalCleanupReceipt {
    pub(crate) step: RemovalCleanupStep,
    pub(crate) disposition: RemovalCleanupDisposition,
    pub(crate) completed_at_ms: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemovalExecutionState {
    pub(crate) remote_import_plan: Option<RemoteImportRepoRemovalPlan>,
    pub(crate) remote_import_checkpoint: Option<RemoteImportRepoRemovalCheckpoint>,
    pub(crate) notegit_checkpoint: Option<NotegitRemovalCheckpoint>,
    pub(crate) authority_checkpoint: Option<RepoAuthorityDatabaseCheckpoint>,
    pub(crate) cut: RemovalCutState,
    pub(crate) cleanup: Vec<RemovalCleanupReceipt>,
    pub(crate) cleanup_complete: bool,
    pub(crate) tombstone_retired: bool,
    pub(crate) terminal: RemovalTerminalState,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RemovalCutState {
    #[default]
    NotAttempted,
    Attempted,
    Observed {
        tombstone: RepoCatalogMembershipRecord,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RemovalTerminalState {
    #[default]
    None,
    Candidate {
        completion: Box<RepoLifecycleJobCompletion>,
    },
    Complete,
}

impl RemovalExecutionState {
    pub(crate) fn completed(&self, step: RemovalCleanupStep) -> bool {
        self.cleanup
            .iter()
            .any(|receipt| receipt.step == step && receipt.disposition.is_success())
    }

    pub(crate) fn has_committed_debt(&self) -> bool {
        !matches!(self.cut, RemovalCutState::NotAttempted)
            && !matches!(self.terminal, RemovalTerminalState::Complete)
    }

    pub(crate) fn tombstone(&self) -> Option<&RepoCatalogMembershipRecord> {
        match &self.cut {
            RemovalCutState::Observed { tombstone } => Some(tombstone),
            RemovalCutState::NotAttempted | RemovalCutState::Attempted => None,
        }
    }

    pub(crate) fn terminal_candidate(&self) -> Option<&RepoLifecycleJobCompletion> {
        match &self.terminal {
            RemovalTerminalState::Candidate { completion } => Some(completion),
            RemovalTerminalState::None | RemovalTerminalState::Complete => None,
        }
    }

    pub(super) fn validate(
        &self,
        request_id: Uuid,
        manifest_digest: &str,
        receipt: &LifecycleReceipt,
    ) -> Result<(), RepoLifecycleJobError> {
        if !matches!(self.cut, RemovalCutState::NotAttempted) && self.remote_import_plan.is_none() {
            return Err(store_invalid(
                "removal cut has no sealed Remote Import plan",
            ));
        }
        if self.remote_import_checkpoint.is_some() && self.remote_import_plan.is_none() {
            return Err(store_invalid(
                "Remote Import checkpoint has no sealed owner plan",
            ));
        }
        if (self.notegit_checkpoint.is_some() || self.authority_checkpoint.is_some())
            && !matches!(self.cut, RemovalCutState::Observed { .. })
        {
            return Err(store_invalid(
                "destructive owner checkpoint preceded the removal cut",
            ));
        }
        if let Some(tombstone) = self.tombstone()
            && !tombstone.confirms_removed_manifest(request_id, manifest_digest)
        {
            return Err(store_invalid(
                "removal tombstone does not bind exact manifest",
            ));
        }
        if self.cleanup.len() > RemovalCleanupStep::ORDER.len() {
            return Err(store_invalid(
                "removal cleanup receipt count exceeds owner plan",
            ));
        }
        for (index, cleanup) in self.cleanup.iter().enumerate() {
            if cleanup.step != RemovalCleanupStep::ORDER[index] || cleanup.completed_at_ms <= 0 {
                return Err(store_invalid(
                    "removal cleanup receipts are not a strict prefix",
                ));
            }
        }
        if !self.cleanup.is_empty() && !matches!(self.cut, RemovalCutState::Observed { .. }) {
            return Err(store_invalid("removal cleanup started before durable cut"));
        }
        if self.cleanup_complete
            && (self.cleanup.len() != RemovalCleanupStep::ORDER.len()
                || self
                    .cleanup
                    .iter()
                    .any(|receipt| !receipt.disposition.is_success()))
        {
            return Err(store_invalid("CleanupComplete lacks every owner receipt"));
        }
        if self.tombstone_retired && !self.cleanup_complete {
            return Err(store_invalid(
                "catalog tombstone retired before CleanupComplete",
            ));
        }
        match &self.terminal {
            RemovalTerminalState::None => {}
            RemovalTerminalState::Candidate { completion } => {
                if !self.tombstone_retired
                    || receipt.phase.is_terminal()
                    || receipt.publication_pending
                    || completion.publication.is_none()
                {
                    return Err(store_invalid(
                        "terminal candidate is not publication-disabled committed settlement",
                    ));
                }
            }
            RemovalTerminalState::Complete => {
                if !self.tombstone_retired || !receipt.phase.is_terminal() {
                    return Err(store_invalid(
                        "terminal removal result precedes tombstone retirement",
                    ));
                }
            }
        }
        Ok(())
    }
}
