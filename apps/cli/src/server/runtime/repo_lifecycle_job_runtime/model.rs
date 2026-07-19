//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 07_network#repo-control-wire-contract

use deve_core::ledger::HostRepoAliasRuntime;
use deve_core::models::RepoId;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use uuid::Uuid;

pub(super) type JobFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct RepoLifecycleJobIntent {
    inner: RepoLifecycleJobIntentKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum RepoLifecycleJobIntentKind {
    Create {
        initial_alias: String,
        projection_base: PathBuf,
    },
    Remove {
        repo_id: RepoId,
    },
}

impl RepoLifecycleJobIntent {
    pub(crate) fn create(
        initial_alias: &str,
        projection_base: impl AsRef<Path>,
    ) -> Result<Self, RepoLifecycleJobError> {
        let initial_alias = HostRepoAliasRuntime::normalize_alias(initial_alias)
            .map_err(|_| RepoLifecycleJobError::InvalidRequest)?;
        let projection_base = projection_base.as_ref();
        if !projection_base.is_absolute() {
            return Err(RepoLifecycleJobError::InvalidRequest);
        }
        Ok(Self {
            inner: RepoLifecycleJobIntentKind::Create {
                initial_alias,
                projection_base: projection_base.to_path_buf(),
            },
        })
    }

    pub(crate) const fn remove(repo_id: RepoId) -> Self {
        Self {
            inner: RepoLifecycleJobIntentKind::Remove { repo_id },
        }
    }

    pub(crate) const fn operation(&self) -> RepoLifecycleJobOperation {
        match &self.inner {
            RepoLifecycleJobIntentKind::Create { .. } => RepoLifecycleJobOperation::Create,
            RepoLifecycleJobIntentKind::Remove { .. } => RepoLifecycleJobOperation::Remove,
        }
    }

    pub(super) const fn requested_repo_id(&self) -> Option<RepoId> {
        match &self.inner {
            RepoLifecycleJobIntentKind::Create { .. } => None,
            RepoLifecycleJobIntentKind::Remove { repo_id } => Some(*repo_id),
        }
    }

    pub(super) fn validate(&self) -> Result<(), RepoLifecycleJobError> {
        match &self.inner {
            RepoLifecycleJobIntentKind::Create {
                initial_alias,
                projection_base,
            } => {
                let normalized = HostRepoAliasRuntime::normalize_alias(initial_alias)
                    .map_err(|_| RepoLifecycleJobError::InvalidRequest)?;
                if normalized != *initial_alias || !projection_base.is_absolute() {
                    return Err(RepoLifecycleJobError::InvalidRequest);
                }
                Ok(())
            }
            RepoLifecycleJobIntentKind::Remove { .. } => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepoLifecycleJobOperation {
    Create,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepoLifecycleJobPhase {
    Accepted,
    Running,
    Recovering,
    Terminal,
}

impl RepoLifecycleJobPhase {
    pub(super) const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepoLifecycleJobOutcome {
    Succeeded,
    NotCommitted,
    CommittedPartial,
    RepairRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RepoLifecycleSettledPublication {
    Created {
        repo_id: RepoId,
        mounted: bool,
    },
    Removed {
        repo_id: RepoId,
        fallback_repo_id: Option<RepoId>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdmittedRepoLifecycleJob {
    pub(crate) request_id: Uuid,
    pub(crate) job_id: Uuid,
    pub(crate) target_repo_id: RepoId,
    pub(crate) intent: RepoLifecycleJobIntent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoLifecycleJobCompletion {
    pub(crate) outcome: RepoLifecycleJobOutcome,
    pub(crate) publication: Option<RepoLifecycleSettledPublication>,
    pub(crate) primary: Option<String>,
    pub(crate) cleanup: Vec<String>,
}

impl RepoLifecycleJobCompletion {
    pub(crate) fn succeeded(publication: RepoLifecycleSettledPublication) -> Self {
        Self {
            outcome: RepoLifecycleJobOutcome::Succeeded,
            publication: Some(publication),
            primary: None,
            cleanup: Vec::new(),
        }
    }

    pub(crate) fn not_committed(primary: impl Into<String>) -> Self {
        Self::failed(RepoLifecycleJobOutcome::NotCommitted, primary)
    }

    pub(crate) fn committed_partial(primary: impl Into<String>) -> Self {
        Self::failed(RepoLifecycleJobOutcome::CommittedPartial, primary)
    }

    pub(crate) fn repair_required(primary: impl Into<String>) -> Self {
        Self::failed(RepoLifecycleJobOutcome::RepairRequired, primary)
    }

    fn failed(outcome: RepoLifecycleJobOutcome, primary: impl Into<String>) -> Self {
        Self {
            outcome,
            publication: None,
            primary: Some(primary.into()),
            cleanup: Vec::new(),
        }
    }

    pub(crate) fn with_cleanup(mut self, cleanup: impl Into<String>) -> Self {
        self.cleanup.push(cleanup.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoLifecycleJobAccepted {
    pub(crate) request_id: Uuid,
    pub(crate) job_id: Uuid,
    pub(crate) target_repo_id: RepoId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepoLifecycleJobStatus {
    pub(crate) request_id: Uuid,
    pub(crate) job_id: Uuid,
    pub(crate) target_repo_id: RepoId,
    pub(crate) operation: RepoLifecycleJobOperation,
    pub(crate) phase: RepoLifecycleJobPhase,
    pub(crate) outcome: Option<RepoLifecycleJobOutcome>,
    pub(crate) publication_pending: bool,
}

pub(crate) trait RepoLifecycleJobExecutor: Send + Sync + 'static {
    fn execute(&self, job: AdmittedRepoLifecycleJob) -> JobFuture<RepoLifecycleJobCompletion>;

    fn recover(&self, job: AdmittedRepoLifecycleJob) -> JobFuture<RepoLifecycleJobCompletion>;

    fn retain_create_receipt(&self, _repo_id: RepoId) -> bool {
        false
    }
}

pub(crate) trait RepoLifecyclePublicationSink: Send + Sync + 'static {
    fn publish(
        &self,
        request_id: Uuid,
        publication: RepoLifecycleSettledPublication,
    ) -> JobFuture<Result<(), String>>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RepoLifecycleJobError {
    #[error("repository lifecycle admission is closed")]
    AdmissionClosed,
    #[error("repository lifecycle request is invalid")]
    InvalidRequest,
    #[error("request_id is already bound to different lifecycle parameters")]
    RequestConflict,
    #[error("repository lifecycle runtime is busy")]
    Busy,
    #[error("repository lifecycle request was not found")]
    NotFound,
    #[error("repository lifecycle receipt store failed: {0}")]
    Store(String),
    #[error("repository lifecycle runtime coordination failed: {0}")]
    Coordination(&'static str),
    #[error("repository lifecycle shutdown failed: {0}")]
    Shutdown(String),
}

impl From<std::io::Error> for RepoLifecycleJobError {
    fn from(error: std::io::Error) -> Self {
        Self::Store(error.to_string())
    }
}

impl From<serde_json::Error> for RepoLifecycleJobError {
    fn from(error: serde_json::Error) -> Self {
        Self::Store(error.to_string())
    }
}
