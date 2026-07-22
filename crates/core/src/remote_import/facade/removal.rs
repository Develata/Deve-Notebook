//! plan_ref:
//!   - 04_repository#remote-import-repo-lifecycle
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 06_backup#remote-import-state-machine
//!   - 06_backup#remote-import-removal-owner-plan
//!
//! Typed repo-removal admission. The Remote Import runtime, rather than a CLI
//! caller, owns the active-session and cleanup-debt classification.

use super::super::{
    RemoteImportProjectionOutcome, RemoteImportResult, RemoteImportSessionId, RemoteImportState,
};
use super::RemoteImportService;
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteImportRepoRemovalBlocker {
    ProjectionPending { session_id: RemoteImportSessionId },
    ProjectionDegraded { session_id: RemoteImportSessionId },
}

/// Opaque exact observation of the repo-local Remote Import workflow. The
/// generation is intentionally not exposed as product or durable identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteImportRepoRemovalSnapshot {
    repo_id: RepoId,
    runtime_generation: u64,
    capture_cleanup_required: bool,
    observation_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteImportRepoRemovalPlan {
    snapshot: RemoteImportRepoRemovalSnapshot,
    artifact: super::super::artifact::RemoteImportArtifactRemovalPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteImportRepoRemovalCheckpoint {
    artifact: super::super::artifact::RemoteImportArtifactRemovalCheckpoint,
}

impl RemoteImportRepoRemovalPlan {
    pub fn initial_checkpoint(&self) -> RemoteImportRepoRemovalCheckpoint {
        RemoteImportRepoRemovalCheckpoint {
            artifact:
                super::super::artifact::RemoteImportArtifactRoot::initial_repo_removal_checkpoint(),
        }
    }
}

impl RemoteImportRepoRemovalCheckpoint {
    pub fn is_complete(&self) -> bool {
        self.artifact.is_complete()
    }
}

impl RemoteImportRepoRemovalSnapshot {
    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub const fn capture_cleanup_required(&self) -> bool {
        self.capture_cleanup_required
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteImportRepoRemovalBlocked {
    repo_id: RepoId,
    blockers: Vec<RemoteImportRepoRemovalBlocker>,
}

impl RemoteImportRepoRemovalBlocked {
    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn blockers(&self) -> &[RemoteImportRepoRemovalBlocker] {
        &self.blockers
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteImportRepoRemovalAdmission {
    Admitted(RemoteImportRepoRemovalSnapshot),
    Blocked(RemoteImportRepoRemovalBlocked),
}

impl RemoteImportRepoRemovalAdmission {
    pub const fn repo_id(&self) -> RepoId {
        match self {
            Self::Admitted(snapshot) => snapshot.repo_id,
            Self::Blocked(blocked) => blocked.repo_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteImportRepoRemovalRevalidation {
    Exact,
    Changed(RemoteImportRepoRemovalAdmission),
}

impl RemoteImportService {
    pub fn repo_removal_admission(&self) -> RemoteImportResult<RemoteImportRepoRemovalAdmission> {
        let (runtime_generation, records) = self.inner.repo_removal_observation()?;
        let mut blockers = Vec::new();
        let capture_cleanup_required = records
            .iter()
            .any(|record| !record.state.is_terminal() || record.cleanup_pending);
        for record in &records {
            if record.state == RemoteImportState::Applied {
                match record
                    .apply_receipt
                    .as_ref()
                    .map(|receipt| receipt.projection_outcome)
                {
                    Some(RemoteImportProjectionOutcome::Pending) => {
                        blockers.push(RemoteImportRepoRemovalBlocker::ProjectionPending {
                            session_id: record.session_id,
                        });
                    }
                    Some(RemoteImportProjectionOutcome::Degraded) => {
                        blockers.push(RemoteImportRepoRemovalBlocker::ProjectionDegraded {
                            session_id: record.session_id,
                        });
                    }
                    Some(RemoteImportProjectionOutcome::Written) | None => {}
                }
            }
        }
        let snapshot = RemoteImportRepoRemovalSnapshot {
            repo_id: self.repo_id,
            runtime_generation,
            capture_cleanup_required,
            observation_digest: format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&records).map_err(|error| {
                    super::super::RemoteImportError::Storage(error.to_string())
                })?)
            ),
        };
        if blockers.is_empty() {
            Ok(RemoteImportRepoRemovalAdmission::Admitted(snapshot))
        } else {
            Ok(RemoteImportRepoRemovalAdmission::Blocked(
                RemoteImportRepoRemovalBlocked {
                    repo_id: self.repo_id,
                    blockers,
                },
            ))
        }
    }

    pub fn revalidate_repo_removal(
        &self,
        expected: &RemoteImportRepoRemovalSnapshot,
    ) -> RemoteImportResult<RemoteImportRepoRemovalRevalidation> {
        let current = self.repo_removal_admission()?;
        if matches!(
            &current,
            RemoteImportRepoRemovalAdmission::Admitted(snapshot) if snapshot == expected
        ) {
            Ok(RemoteImportRepoRemovalRevalidation::Exact)
        } else {
            Ok(RemoteImportRepoRemovalRevalidation::Changed(current))
        }
    }

    pub fn seal_repo_removal(
        &self,
        expected: &RemoteImportRepoRemovalSnapshot,
    ) -> RemoteImportResult<RemoteImportRepoRemovalPlan> {
        if self.revalidate_repo_removal(expected)? != RemoteImportRepoRemovalRevalidation::Exact {
            return Err(super::super::RemoteImportError::RepoRemovalChanged(
                self.repo_id,
            ));
        }
        Ok(RemoteImportRepoRemovalPlan {
            snapshot: expected.clone(),
            artifact: self.inner.artifacts.seal_repo_removal(self.repo_id)?,
        })
    }

    pub fn revalidate_sealed_repo_removal(
        &self,
        expected: &RemoteImportRepoRemovalSnapshot,
        plan: &RemoteImportRepoRemovalPlan,
    ) -> RemoteImportResult<bool> {
        if plan.snapshot != *expected
            || self.revalidate_repo_removal(expected)? != RemoteImportRepoRemovalRevalidation::Exact
        {
            return Ok(false);
        }
        super::super::artifact::RemoteImportArtifactRoot::revalidate_repo_removal(&plan.artifact)
    }

    pub fn invalidate_repo_removal(plan: &RemoteImportRepoRemovalPlan) -> RemoteImportResult<()> {
        super::super::artifact::RemoteImportArtifactRoot::invalidate_repo_removal(&plan.artifact)
    }

    pub fn advance_repo_removal(
        plan: &RemoteImportRepoRemovalPlan,
        checkpoint: &RemoteImportRepoRemovalCheckpoint,
    ) -> RemoteImportResult<RemoteImportRepoRemovalCheckpoint> {
        Ok(RemoteImportRepoRemovalCheckpoint {
            artifact: super::super::artifact::RemoteImportArtifactRoot::advance_repo_removal(
                &plan.artifact,
                &checkpoint.artifact,
            )?,
        })
    }

    pub fn verify_repo_removal_complete(
        plan: &RemoteImportRepoRemovalPlan,
        checkpoint: &RemoteImportRepoRemovalCheckpoint,
    ) -> RemoteImportResult<()> {
        super::super::artifact::RemoteImportArtifactRoot::verify_repo_removal_complete(
            &plan.artifact,
            &checkpoint.artifact,
        )
    }
}
