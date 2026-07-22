//! plan_ref:
//!   - 04_repository#remote-import-repo-lifecycle
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 06_backup#remote-import-state-machine
//!
//! Typed repo-removal admission. The Remote Import runtime, rather than a CLI
//! caller, owns the active-session and cleanup-debt classification.

use super::super::{
    RemoteImportProjectionOutcome, RemoteImportResult, RemoteImportSessionId, RemoteImportState,
};
use super::RemoteImportService;
use crate::models::RepoId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteImportRepoRemovalBlocker {
    ProjectionPending { session_id: RemoteImportSessionId },
    ProjectionDegraded { session_id: RemoteImportSessionId },
}

/// Opaque exact observation of the repo-local Remote Import workflow. The
/// generation is intentionally not exposed as product or durable identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteImportRepoRemovalSnapshot {
    repo_id: RepoId,
    runtime_generation: u64,
    capture_cleanup_required: bool,
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
}
